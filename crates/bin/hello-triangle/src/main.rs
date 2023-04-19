use anyhow::Result;
use cinder::{
    command_queue::RenderAttachment,
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        pipeline::graphics::GraphicsPipeline,
    },
    Cinder,
};
use math::{mat::Mat4, vec::Vec3};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/triangle_shader_structs.rs"
));

pub struct HelloTriangle {
    cinder: Cinder,
    pipeline: GraphicsPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl HelloTriangle {
    pub fn new(window: &Window) -> Result<Self> {
        let (width, height) = window.drawable_size();
        let cinder = Cinder::new(window, width, height)?;

        let vertex_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/triangle.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = cinder.device.create_shader(
            include_bytes!("../shaders/spv/triangle.frag.spv"),
            Default::default(),
        )?;
        let pipeline = cinder.device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            Default::default(),
        )?;

        let vertex_buffer = cinder.device.create_buffer_with_data(
            &[
                TriangleVertex {
                    i_pos: [0.0, 0.5],
                    i_color: [1.0, 0.0, 0.0, 1.0],
                },
                TriangleVertex {
                    i_pos: [-0.5, -0.5],
                    i_color: [0.0, 1.0, 0.0, 1.0],
                },
                TriangleVertex {
                    i_pos: [0.5, -0.5],
                    i_color: [0.0, 0.0, 1.0, 1.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = cinder.device.create_buffer_with_data(
            &[0, 1, 2],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        vertex_shader.destroy(&cinder.device);
        fragment_shader.destroy(&cinder.device);

        Ok(Self {
            cinder,
            pipeline,
            vertex_buffer,
            index_buffer,
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
        cmd_list.set_vertex_bytes(
            &self.cinder.device,
            &self.pipeline,
            &Mat4::rotate(
                (self.cinder.init_time.elapsed().as_secs_f32() / 5.0)
                    * (2.0 * std::f32::consts::PI),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            0,
        )?;
        cmd_list.draw_offset(&self.cinder.device, 3, 0, 0);
        cmd_list.end_rendering(&self.cinder.device);

        self.cinder
            .swapchain
            .present(&self.cinder.device, cmd_list, swapchain_image)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.cinder.resize(width, height)
    }
}

impl Drop for HelloTriangle {
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
            title: "hello-triangle",
        },
    )
    .unwrap();

    let mut renderer = HelloTriangle::new(&sdl.window).unwrap();

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
