use anyhow::Result;
use cinder::{
    cinder::Cinder,
    command_queue::{AttachmentStoreOp, ClearValue, RenderAttachmentDesc},
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
    },
    ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use render_graph::{AttachmentType, RenderGraph, RenderPass};
use scene::{ObjMesh, Scene, Vertex};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use std::path::PathBuf;
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/mesh_shader_structs.rs"
));

impl Vertex for MeshVertex {
    fn from_obj_mesh_index(mesh: &ObjMesh, i: usize) -> Self {
        let i_pos = [
            mesh.positions[i * 3],
            mesh.positions[i * 3 + 1],
            mesh.positions[i * 3 + 2],
        ];

        let i_uv = if !mesh.texcoords.is_empty() {
            [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
        } else {
            [0.0, 0.0]
        };

        Self { i_pos, i_uv }
    }

    fn pos_3d(&self) -> [f32; 3] {
        self.i_pos
    }

    fn set_pos_3d(mut self, x: f32, y: f32, z: f32) -> Self {
        self.i_pos = [x, y, z];
        self
    }
}

pub struct Renderer {
    cinder: Cinder,
    index_count: u32,
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    depth_image_handle: ResourceId<Image>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let mut cinder = Cinder::new(window, width, height)?;

        //
        // Create App Resources
        //
        let surface_rect = cinder.device.surface_rect();
        let depth_image = cinder.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/mesh.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/mesh.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;

        let ubo_buffer = cinder.device.create_buffer(
            std::mem::size_of::<MeshUniformBufferObject>() as u64,
            BufferDescription {
                usage: BufferUsage::UNIFORM,
                ..Default::default()
            },
        )?;
        ubo_buffer.mem_copy(
            util::offset_of!(MeshUniformBufferObject, view) as u64,
            &[
                camera::look_to(
                    Vec3::new(2.0, -0.5, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ),
                camera::new_infinite_perspective_proj(
                    surface_rect.width() as f32 / surface_rect.height() as f32,
                    30.0,
                    0.01,
                ),
            ],
        )?;

        let sampler = cinder.device.create_sampler(Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/textures/viking_room.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = cinder.device.create_image_with_data_immediate(
            Size2D::new(width, height),
            &image_data,
            &cinder.command_queue,
            Default::default(),
        )?;
        cinder.device.write_bind_group(&[
            BindGroupBindInfo {
                group: bind_group,
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
            },
            BindGroupBindInfo {
                group: bind_group,
                dst_binding: 1,
                data: BindGroupWriteData::SampledImage(texture.bind_info(
                    &sampler,
                    Layout::ShaderReadOnly,
                    None,
                )),
            },
        ])?;

        let scene = Scene::<MeshVertex>::from_obj(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("models"),
            "viking_room.obj",
        )?;
        let mesh = scene.meshes.first().unwrap();
        let vertex_buffer = cinder.device.create_buffer_with_data(
            &mesh.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = cinder.device.create_buffer_with_data(
            &mesh.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        //
        // Add resources to ResourceManager
        //
        cinder.resource_manager.insert_sampler(sampler);
        cinder.resource_manager.insert_image(texture);

        //
        // Cleanup
        //
        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        let depth_image_handle = cinder.resource_manager.insert_image(depth_image);

        Ok(Self {
            cinder,
            index_count: mesh.indices.len() as u32,
            depth_image_handle,
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale =
            (self.cinder.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        self.ubo_buffer.mem_copy(
            util::offset_of!(MeshUniformBufferObject, model) as u64,
            &[
                Mat4::rotate(std::f32::consts::PI / 2.0, Vec3::new(1.0, 0.0, 0.0))
                    * Mat4::rotate(scale, Vec3::new(0.0, 0.0, 1.0)),
            ],
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<bool> {
        let mut graph = RenderGraph::new();
        graph.add_pass(
            RenderPass::default()
                .add_color_attachment(AttachmentType::SwapchainImage, Default::default())
                .set_depth_attachment(
                    AttachmentType::Reference(self.depth_image_handle),
                    RenderAttachmentDesc {
                        store_op: AttachmentStoreOp::DontCare,
                        layout: Layout::DepthAttachment,
                        clear_value: ClearValue::default_depth(),
                        ..Default::default()
                    },
                )
                .set_callback(|cinder, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&cinder.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&cinder.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&cinder.device, &self.vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &cinder.device,
                        &self.pipeline,
                        0,
                        &[self.bind_group],
                    );
                    cmd_list.draw_offset(&cinder.device, self.index_count, 0, 0);
                    Ok(())
                }),
        );

        graph.run(&mut self.cinder)?.present(&mut self.cinder)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)?;
        let depth_image = self
            .cinder
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&self.cinder.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.index_buffer.destroy(&self.cinder.device);
        self.vertex_buffer.destroy(&self.cinder.device);
        self.ubo_buffer.destroy(&self.cinder.device);
        self.pipeline.destroy(&self.cinder.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "mesh",
            ..Default::default()
        },
    )
    .unwrap();

    let mut renderer = Renderer::new(&sdl.window).unwrap();

    'running: loop {
        renderer.cinder.start_frame().unwrap();

        for event in sdl.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::SizeChanged(width, height),
                    ..
                } => {
                    renderer.resize(width as u32, height as u32).unwrap();
                }
                _ => {}
            }
        }

        renderer.update().unwrap();
        renderer.draw().unwrap();

        renderer.cinder.end_frame();
    }
}
