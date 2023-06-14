use std::path::PathBuf;

use anyhow::Result;
use cinder::{
    cinder::Cinder,
    command_queue::RenderAttachment,
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::Layout,
        pipeline::graphics::GraphicsPipeline,
    },
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
    cinder: Cinder,
    pipeline: GraphicsPipeline,
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
        // Create App Resources
        //
        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/texture.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/texture.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            Some(&fragment_shader),
            Default::default(),
        )?;
        let bind_group = BindGroup::new(&cinder.device, pipeline.bind_group_data(0).unwrap())?;
        let sampler = cinder.device.create_sampler(Default::default())?;
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
        let texture = cinder.device.create_image_with_data_immediate(
            Size2D::new(image_data.width, image_data.height),
            &image_data.bytes,
            &cinder.command_queue,
            Default::default(),
        )?;
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

        //
        // Cleanup
        //
        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let surface_rect = self.cinder.device.surface_rect();

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
        cmd_list.bind_graphics_pipeline(&self.cinder.device, &self.pipeline);
        cmd_list.bind_viewport(&self.cinder.device, surface_rect, true);
        cmd_list.bind_scissor(&self.cinder.device, surface_rect);
        cmd_list.bind_index_buffer(&self.cinder.device, &self.index_buffer);
        cmd_list.bind_vertex_buffer(&self.cinder.device, &self.vertex_buffer);
        cmd_list.bind_descriptor_sets(&self.cinder.device, &self.pipeline, 0, &[self.bind_group]);
        cmd_list.draw_offset(&self.cinder.device, 6, 0, 0);
        cmd_list.end_rendering(&self.cinder.device);

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.cinder.device.wait_idle().ok();
        self.index_buffer.destroy(&self.cinder.device);
        self.vertex_buffer.destroy(&self.cinder.device);
        self.pipeline.destroy(&self.cinder.device);
    }
}

fn main() {
    let mut sdl = SdlContext::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowDescription {
            title: "texture",
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
        renderer.draw().unwrap();

        renderer.cinder.end_frame();
    }
}
