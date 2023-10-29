use anyhow::Result;
use cinder::{
    App, AttachmentStoreOp, AttachmentType, BindGroup, BindGroupBindInfo, BindGroupWriteData,
    Buffer, BufferDescription, BufferUsage, Bump, Cinder, ClearValue, Format, GraphicsPipeline,
    GraphicsPipelineDescription, Image, ImageDescription, ImageUsage, Layout, RenderAttachmentDesc,
    RenderGraph, RenderPass, Renderer, ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use scene::{ObjMesh, Scene, Vertex};

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

pub struct MeshSample {
    index_count: u32,
    pipeline: GraphicsPipeline,
    bind_group: BindGroup,
    depth_image_handle: ResourceId<Image>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    ubo_buffer: Buffer,
}

impl App for MeshSample {
    fn new(renderer: &mut Renderer, _width: u32, _height: u32) -> Result<Self> {
        //
        // Create App Resources
        //
        let surface_rect = renderer.device.surface_rect();
        let depth_image = renderer.device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFLOAT,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/mesh.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = renderer.device.create_shader(
            include_bytes!("../shaders/spv/mesh.frag.spv"),
            Default::default(),
        )?;
        let pipeline = renderer.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFLOAT),
                ..Default::default()
            },
        )?;
        let bind_group = BindGroup::new(&renderer.device, pipeline.bind_group_data(0).unwrap())?;

        let ubo_buffer = renderer.device.create_buffer(
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

        let sampler = renderer.device.create_sampler(Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/textures/viking_room.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = renderer.device.create_image_with_data_immediate(
            Size2D::new(width, height),
            &image_data,
            &renderer.command_queue,
            Default::default(),
        )?;
        renderer.device.write_bind_group(&[
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
        let vertex_buffer = renderer.device.create_buffer_with_data(
            &mesh.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = renderer.device.create_buffer_with_data(
            &mesh.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        //
        // Add resources to ResourceManager
        //
        renderer.resource_manager.insert_sampler(sampler);
        renderer.resource_manager.insert_image(texture);

        //
        // Cleanup
        //
        vertex_shader.destroy(&renderer.device);
        fragment_shader.destroy(&renderer.device);

        let depth_image_handle = renderer.resource_manager.insert_image(depth_image);

        Ok(Self {
            index_count: mesh.indices.len() as u32,
            depth_image_handle,
            pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            ubo_buffer,
        })
    }

    fn update(&mut self, renderer: &mut Renderer) -> Result<()> {
        let scale =
            (renderer.init_time().elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        self.ubo_buffer.mem_copy(
            util::offset_of!(MeshUniformBufferObject, model) as u64,
            &[
                Mat4::rotate(std::f32::consts::PI / 2.0, Vec3::new(1.0, 0.0, 0.0))
                    * Mat4::rotate(scale, Vec3::new(0.0, 0.0, 1.0)),
            ],
        )?;
        Ok(())
    }

    fn draw<'a>(
        &'a mut self,
        allocator: &'a Bump,
        graph: &mut RenderGraph<'a>,
    ) -> anyhow::Result<()> {
        graph.add_pass(
            allocator,
            RenderPass::new(allocator)
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
                .set_callback(allocator, |renderer, cmd_list| {
                    cmd_list.bind_graphics_pipeline(&renderer.device, &self.pipeline);
                    cmd_list.bind_index_buffer(&renderer.device, &self.index_buffer);
                    cmd_list.bind_vertex_buffer(&renderer.device, &self.vertex_buffer);
                    cmd_list.bind_descriptor_sets(
                        &renderer.device,
                        &self.pipeline,
                        0,
                        &[self.bind_group],
                    );
                    cmd_list.draw_offset(&renderer.device, self.index_count, 0, 0);
                    Ok(())
                }),
        );
        Ok(())
    }

    fn resize(&mut self, renderer: &mut Renderer, width: u32, height: u32) -> Result<()> {
        renderer.resize(width, height)?;
        let depth_image = renderer
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&renderer.device, Size2D::new(width, height))?;
        Ok(())
    }

    fn cleanup(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        self.index_buffer.destroy(&renderer.device);
        self.vertex_buffer.destroy(&renderer.device);
        self.ubo_buffer.destroy(&renderer.device);
        self.pipeline.destroy(&renderer.device);
        Ok(())
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
    let mut cinder: Cinder<_> = Cinder::<MeshSample>::new(&sdl.window).unwrap();
    cinder.run_game_loop(&mut sdl).unwrap();
}
