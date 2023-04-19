use anyhow::Result;
use cinder::{
    command_queue::{
        AttachmentStoreOp, ClearValue, CommandQueue, RenderAttachment, RenderAttachmentDesc,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, Image, ImageDescription, ImageUsage, Layout},
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        ResourceManager,
    },
    swapchain::Swapchain,
    ResourceId,
};
use math::{mat::Mat4, size::Size2D, vec::Vec3};
use scene::{ObjMesh, Scene, Vertex};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use std::{path::PathBuf, time::Instant};
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
    resource_manager: ResourceManager,
    device: Device,
    swapchain: Swapchain,
    command_queue: CommandQueue,
    init_time: Instant,
    index_count: u32,
    render_pipeline_handle: ResourceId<GraphicsPipeline>,
    depth_image_handle: ResourceId<Image>,
    vertex_buffer_handle: ResourceId<Buffer>,
    index_buffer_handle: ResourceId<Buffer>,
    ubo_buffer_handle: ResourceId<Buffer>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let mut resource_manager = ResourceManager::default();
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height)?;
        let command_queue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device)?;

        //
        // Create App Resources
        //
        let surface_rect = device.surface_rect();
        let depth_image = device.create_image(
            Size2D::new(surface_rect.width(), surface_rect.height()),
            ImageDescription {
                format: Format::D32_SFloat,
                usage: ImageUsage::Depth,
                ..Default::default()
            },
        )?;
        let vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/mesh.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/mesh.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            GraphicsPipelineDescription {
                depth_format: Some(Format::D32_SFloat),
                ..Default::default()
            },
        )?;

        let mut ubo_buffer = device.create_buffer(
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

        let sampler = device.create_sampler(&device, Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/textures/viking_room.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = device.create_image_with_data(
            Size2D::new(width, height),
            &image_data,
            &command_queue,
            Default::default(),
        )?;
        device.write_bind_group(
            &render_pipeline,
            &[
                BindGroupBindInfo {
                    dst_binding: 0,
                    data: BindGroupWriteData::Uniform(ubo_buffer.bind_info()),
                },
                BindGroupBindInfo {
                    dst_binding: 1,
                    data: BindGroupWriteData::SampledImage(texture.bind_info(
                        &sampler,
                        Layout::ShaderReadOnly,
                        None,
                    )),
                },
            ],
        )?;

        let scene = Scene::<MeshVertex>::from_obj(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("models"),
            "viking_room.obj",
        )?;
        let mesh = scene.meshes.first().unwrap();

        //
        // Add resources to ResourceManager
        //
        let vertex_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &mesh.vertices,
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?);
        let index_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &mesh.indices,
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);
        let ubo_buffer_handle = resource_manager.insert_buffer(ubo_buffer);
        let render_pipeline_handle = resource_manager.insert_graphics_pipeline(render_pipeline);
        let depth_image_handle = resource_manager.insert_image(depth_image);
        resource_manager.insert_sampler(sampler);
        resource_manager.insert_image(texture);

        //
        // Cleanup
        //
        vertex_shader.destroy(device.raw());
        fragment_shader.destroy(device.raw());

        Ok(Self {
            resource_manager,
            device,
            swapchain,
            command_queue,
            init_time: Instant::now(),
            index_count: mesh.indices.len() as u32,
            depth_image_handle,
            render_pipeline_handle,
            vertex_buffer_handle,
            index_buffer_handle,
            ubo_buffer_handle,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let scale = (self.init_time.elapsed().as_secs_f32() / 5.0) * (2.0 * std::f32::consts::PI);
        let ubo_buffer = self
            .resource_manager
            .buffers
            .get_mut(self.ubo_buffer_handle)
            .unwrap();
        ubo_buffer.mem_copy(
            util::offset_of!(MeshUniformBufferObject, model) as u64,
            &[
                Mat4::rotate(std::f32::consts::PI / 2.0, Vec3::new(1.0, 0.0, 0.0))
                    * Mat4::rotate(scale, Vec3::new(0.0, 0.0, 1.0)),
            ],
        )?;
        Ok(())
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.device.surface_rect();
        let depth_image = self
            .resource_manager
            .images
            .get(self.depth_image_handle)
            .unwrap();
        let pipeline = self
            .resource_manager
            .graphics_pipelines
            .get(self.render_pipeline_handle)
            .unwrap();
        let index_buffer = self
            .resource_manager
            .buffers
            .get(self.index_buffer_handle)
            .unwrap();
        let vertex_buffer = self
            .resource_manager
            .buffers
            .get(self.vertex_buffer_handle)
            .unwrap();

        let cmd_list = self.command_queue.get_command_list(&self.device)?;
        let swapchain_image = self.swapchain.acquire_image(&self.device, &cmd_list)?;

        cmd_list.begin_rendering(
            &self.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
            Some(RenderAttachment::depth(
                depth_image,
                RenderAttachmentDesc {
                    store_op: AttachmentStoreOp::DontCare,
                    layout: Layout::DepthAttachment,
                    clear_value: ClearValue::default_depth(),
                    ..Default::default()
                },
            )),
        );
        cmd_list.bind_graphics_pipeline(&self.device, pipeline);
        cmd_list.bind_viewport(&self.device, surface_rect, true);
        cmd_list.bind_scissor(&self.device, surface_rect);
        cmd_list.bind_index_buffer(&self.device, index_buffer);
        cmd_list.bind_vertex_buffer(&self.device, vertex_buffer);
        // TODO: re-think API later when using more than one set
        cmd_list.bind_descriptor_sets(&self.device, pipeline);
        cmd_list.draw_offset(&self.device, self.index_count, 0, 0);
        cmd_list.end_rendering(&self.device);

        self.swapchain
            .present(&self.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.swapchain.resize(&self.device)?;
        let depth_image = self
            .resource_manager
            .images
            .get_mut(self.depth_image_handle)
            .unwrap();
        depth_image.resize(&self.device, Size2D::new(width, height))?;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();
        self.command_queue.destroy(&self.device);
        self.swapchain.destroy(&self.device);
        self.resource_manager.force_destroy(&self.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription { title: "mesh" },
    )
    .unwrap();

    let mut renderer = Renderer::new(&sdl.window).unwrap();

    'running: loop {
        renderer.device.new_frame().unwrap();

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

        renderer.resource_manager.consume(&renderer.device);
        renderer.device.bump_frame();
    }
}
