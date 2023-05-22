use std::path::Path;

use anyhow::Result;
use cinder::{
    command_queue::RenderAttachment,
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::Layout,
        pipeline::graphics::GraphicsPipeline,
    },
    Cinder, ResourceId,
};
use math::size::Size2D;
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/hot_reload_shader_structs.rs"
));

pub struct Renderer {
    cinder: Cinder,
    pipeline_handle: ResourceId<GraphicsPipeline>,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        //
        // Create Base Resources
        //
        let (width, height) = window.drawable_size();
        let mut cinder = Cinder::new(window, width, height)?;

        //
        // Setup Shader Hot-reloading
        //
        let vertex_shader_handle =
            cinder
                .resource_manager
                .insert_shader(cinder.device.create_shader(
                    include_bytes!("../shaders/spv/hot_reload.vert.spv"),
                    Default::default(),
                )?);
        let fragment_shader_handle =
            cinder
                .resource_manager
                .insert_shader(cinder.device.create_shader(
                    include_bytes!("../shaders/spv/hot_reload.frag.spv"),
                    Default::default(),
                )?);
        let pipeline = cinder.device.create_graphics_pipeline(
            cinder
                .resource_manager
                .shaders
                .get(vertex_shader_handle)
                .unwrap(),
            cinder.resource_manager.shaders.get(fragment_shader_handle),
            Default::default(),
        )?;
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;
        let pipeline_handle = cinder.resource_manager.insert_graphics_pipeline(pipeline);
        cinder.shader_hot_reloader.set_graphics(
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

        let sampler = cinder.device.create_sampler(Default::default())?;
        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        let texture = cinder.device.create_image_with_data(
            Size2D::new(width, height),
            &image_data,
            &cinder.command_queue,
            Default::default(),
        )?;
        let _pipeline = cinder
            .resource_manager
            .graphics_pipelines
            .get(pipeline_handle)
            .unwrap();
        cinder.device.write_bind_group(&[BindGroupBindInfo {
            group: bind_group,
            dst_binding: 0,
            data: BindGroupWriteData::SampledImage(texture.bind_info(
                &sampler,
                Layout::ShaderReadOnly,
                None,
            )),
        }])?;
        let vertex_buffer = cinder.device.create_buffer_with_data(
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
        )?;
        let index_buffer = cinder.device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
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

        cinder.init();

        Ok(Self {
            cinder,
            pipeline_handle,
            bind_group,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.cinder.device.surface_rect();
        let pipeline = self
            .cinder
            .resource_manager
            .graphics_pipelines
            .get(self.pipeline_handle)
            .unwrap();

        let cmd_list = self
            .cinder
            .command_queue
            .get_command_list(&self.cinder.device)?;
        let swapchain_image = self
            .cinder
            .swapchain
            .acquire_image(&self.cinder.device, &cmd_list)?;

        cmd_list.begin_rendering(
            &self.cinder.device,
            surface_rect,
            &[RenderAttachment::color(swapchain_image, Default::default())],
            None,
        );
        cmd_list.bind_graphics_pipeline(&self.cinder.device, pipeline);
        cmd_list.bind_viewport(&self.cinder.device, surface_rect, true);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);
        cmd_list.bind_index_buffer(&self.cinder.device, &self.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.vertex_buffer);
        cmd_list.bind_descriptor_sets(&self.cinder.device, pipeline, 0, &[self.bind_group]);
        cmd_list.draw_offset(&self.cinder.device, 6, 0, 0);
        cmd_list.end_rendering(&self.cinder.device);

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.device.resize(width, height)?;
        self.cinder.swapchain.resize(&self.cinder.device)?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        for update_data in self.cinder.shader_hot_reloader.drain()? {
            if let Some(pipeline_shader_set) = self
                .cinder
                .shader_hot_reloader
                .get_pipeline(update_data.shader_handle)
            {
                self.cinder.device.recreate_shader(
                    &mut self.cinder.resource_manager,
                    update_data.shader_handle,
                    &update_data.bytes,
                )?;
                self.cinder.device.recreate_graphics_pipeline(
                    &mut self.cinder.resource_manager,
                    pipeline_shader_set.pipeline_handle,
                    pipeline_shader_set.vertex_handle,
                    Some(pipeline_shader_set.fragment_handle),
                )?;
            }
        }
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.vertex_buffer.destroy(&self.cinder.device);
        self.index_buffer.destroy(&self.cinder.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "shader-hot-reload",
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
