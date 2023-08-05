use anyhow::Result;
use cinder::{
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        pipeline::graphics::GraphicsPipeline,
    },
    Cinder,
};
use math::{mat::Mat4, vec::Vec3};
use render_graph::{AttachmentType, RenderGraph};
use sdl2::{event::Event, keyboard::Keycode, video::Window};
use util::{SdlContext, WindowDescription};

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 1280;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/triangle_shader_structs.rs"
));

// TODO: Abstract a lot of this in a `App` trait that will reduce boilerplate

// TODO: a `Cleanup` proc-macro
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
            Some(&fragment_shader),
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
        let mut graph = RenderGraph::new();
        graph
            .register_pass("main_pass")
            .add_color_attachment(AttachmentType::SwapchainImage, Default::default())
            .set_callback(|cinder, cmd_list| {
                cmd_list.bind_graphics_pipeline(&cinder.device, &self.pipeline);
                cmd_list.bind_index_buffer(&cinder.device, &self.index_buffer);
                cmd_list.bind_vertex_buffer(&cinder.device, &self.vertex_buffer);
                cmd_list.set_vertex_bytes(
                    &cinder.device,
                    &self.pipeline,
                    &Mat4::rotate(
                        (cinder.init_time.elapsed().as_secs_f32() / 5.0)
                            * (2.0 * std::f32::consts::PI),
                        Vec3::new(0.0, 0.0, 1.0),
                    ),
                    0,
                )?;
                cmd_list.draw_offset(&cinder.device, 3, 0, 0);

                Ok(())
            });

        graph.run(&mut self.cinder)
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
            ..Default::default()
        },
    )
    .unwrap();

    let mut hello_triangle = HelloTriangle::new(&sdl.window).unwrap();

    'running: loop {
        hello_triangle.cinder.start_frame().unwrap();

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
                    hello_triangle.resize(width as u32, height as u32).unwrap();
                }
                _ => {}
            }
        }

        hello_triangle.draw().unwrap();

        hello_triangle.cinder.end_frame();
    }
}
