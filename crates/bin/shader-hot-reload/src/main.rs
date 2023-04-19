use std::path::Path;

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
use shader_hot_reloader::{ShaderHotReloader, ShaderHotReloaderRunner};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/hot_reload_shader_structs.rs"
));

pub struct Renderer {
    shader_hot_reloader: ShaderHotReloader,
    resource_manager: ResourceManager,
    device: Device,
    swapchain: Swapchain,
    command_queue: CommandQueue,
    pipeline_handle: ResourceId<GraphicsPipeline>,
    vertex_buffer_handle: ResourceId<Buffer>,
    index_buffer_handle: ResourceId<Buffer>,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let mut shader_hot_reloader = ShaderHotReloaderRunner::new()?;
        let (width, height) = window.drawable_size();
        let device = Device::new(window, width, height)?;
        let command_queue = CommandQueue::new(&device)?;
        let swapchain = Swapchain::new(&device)?;
        let vertex_shader_handle = resource_manager.insert_shader(device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.vert.spv"),
            Default::default(),
        )?);
        let fragment_shader_handle = resource_manager.insert_shader(device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.frag.spv"),
            Default::default(),
        )?);
        let pipeline_handle = resource_manager.insert_graphics_pipeline(
            device.create_graphics_pipeline(
                resource_manager.shaders.get(vertex_shader_handle).unwrap(),
                resource_manager
                    .shaders
                    .get(fragment_shader_handle)
                    .unwrap(),
                Default::default(),
            )?,
        );
        shader_hot_reloader.set_graphics(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.vert")
                .canonicalize()?,
            vertex_shader_handle,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.frag")
                .canonicalize()?,
            fragment_shader_handle,
            pipeline_handle,
        )?;
        let sampler = device.create_sampler(&device, Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
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
        let pipeline = resource_manager
            .graphics_pipelines
            .get(pipeline_handle)
            .unwrap();
        device.write_bind_group(
            pipeline,
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
        let vertex_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[
                HotReloadVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                HotReloadVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?);
        let index_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);
        resource_manager.insert_sampler(sampler);
        resource_manager.insert_image(texture);

        Ok(Self {
            shader_hot_reloader: shader_hot_reloader.run(),
            resource_manager,
            device,
            swapchain,
            command_queue,
            pipeline_handle,
            vertex_buffer_handle,
            index_buffer_handle,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.device.surface_rect();
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
        let pipeline = self
            .resource_manager
            .graphics_pipelines
            .get(self.pipeline_handle)
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

    pub fn update(&mut self) -> Result<()> {
        for update_data in self.shader_hot_reloader.drain()? {
            if let Some(pipeline_shader_set) = self
                .shader_hot_reloader
                .get_pipeline(update_data.shader_handle)
            {
                self.device.recreate_shader(
                    &mut self.resource_manager,
                    update_data.shader_handle,
                    &update_data.bytes,
                )?;
                self.device.recreate_graphics_pipeline(
                    &mut self.resource_manager,
                    pipeline_shader_set.pipeline_handle,
                    pipeline_shader_set.vertex_handle,
                    pipeline_shader_set.fragment_handle,
                )?;
            }
        }
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
        WindowDescription {
            title: "shader-hot-reload",
        },
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
