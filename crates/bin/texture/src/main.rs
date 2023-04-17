use std::path::PathBuf;

use anyhow::Result;
use cinder::{
    command_queue::{CommandQueue, RenderAttachment},
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::Layout,
        pipeline::graphics::GraphicsPipeline,
        ResourceManager,
    },
    swapchain::Swapchain,
    ResourceId,
};
use math::size::Size2D;
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/texture_shader_structs.rs"
));

pub struct Renderer {
    device: Device,
    swapchain: Swapchain,
    command_queue: CommandQueue,
    resource_manager: ResourceManager,
    render_pipeline_handle: ResourceId<GraphicsPipeline>,
    vertex_buffer_handle: ResourceId<Buffer>,
    index_buffer_handle: ResourceId<Buffer>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height, Default::default())?;
        let command_queue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device, Default::default())?;

        //
        // Create App Resources
        //
        let vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/texture.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/texture.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            Default::default(),
        )?;
        let sampler = device.create_sampler(&device, Default::default())?;
        let image_data = zero_copy_assets::try_decoded_file::<zero_copy_assets::ImageData>(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("rust.png"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("gen")
                .join("rust.adi"),
        )
        .unwrap();
        let texture = device.create_image_with_data(
            Size2D::new(image_data.width, image_data.height),
            &image_data.bytes,
            &command_queue,
            Default::default(),
        )?;
        device.write_bind_group(
            &render_pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(texture.bind_info(
                    &sampler,
                    Layout::ShaderReadOnly,
                    None,
                )),
            }],
        )?;

        //
        // Add resources to ResourceManager
        //
        let mut resource_manager = ResourceManager::default();
        let render_pipeline_handle = resource_manager.insert_graphics_pipeline(render_pipeline);
        let index_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);
        let vertex_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[
                // Top-left
                TextureVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                // Top-right
                TextureVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                // Bottom-right
                TextureVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                // Bottom-left
                TextureVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?);
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
            render_pipeline_handle,
            vertex_buffer_handle,
            index_buffer_handle,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.device.surface_rect();
        let index_buffer = self
            .resource_manager
            .get_buffer(self.index_buffer_handle)
            .unwrap();
        let vertex_buffer = self
            .resource_manager
            .get_buffer(self.vertex_buffer_handle)
            .unwrap();
        let pipeline = self
            .resource_manager
            .get_graphics_pipeline(self.render_pipeline_handle)
            .unwrap();

        let cmd_list = self.command_queue.get_command_list(&self.device)?;
        let swapchain_image = self.swapchain.acquire_image(&self.device, &cmd_list)?;

        cmd_list.begin_rendering(
            &self.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
            None,
        );
        cmd_list.bind_graphics_pipeline(&self.device, pipeline);
        cmd_list.bind_viewport(&self.device, surface_rect, true);
        cmd_list.bind_scissor(&self.device, surface_rect);
        cmd_list.bind_index_buffer(&self.device, index_buffer);
        cmd_list.bind_vertex_buffer(&self.device, vertex_buffer);
        cmd_list.bind_descriptor_sets(&self.device, pipeline);
        cmd_list.draw_offset(&self.device, 6, 0, 0);
        cmd_list.end_rendering(&self.device);

        self.swapchain
            .present(&self.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.swapchain.resize(&self.device)?;
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
        WindowDescription { title: "ui" },
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
        renderer.draw().unwrap();

        renderer.resource_manager.consume(&renderer.device);
        renderer.device.bump_frame();
    }
}
