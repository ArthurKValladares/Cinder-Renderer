use anyhow::Result;
use cinder::{
    context::render_context::{RenderAttachment, RenderContext},
    device::{Device, SurfaceData},
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        pipeline::graphics::GraphicsPipeline,
    },
    view::View,
    Resolution,
};
use math::{mat::Mat4, rect::Rect2D, vec::Vec3};
use std::time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::VirtualKeyCode,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const WINDOW_WIDTH: u32 = 2000;
pub const WINDOW_HEIGHT: u32 = 2000;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/triangle_shader_structs.rs"
));

pub struct Renderer {
    device: Device,
    view: View,
    render_pipeline: GraphicsPipeline,
    render_context: RenderContext,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    // TODO: Don't need to hold on to all of `SurfaceData`, most of it should be cached in `View`?
    surface_data: SurfaceData,
    init_time: Instant,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let device = Device::new(window)?;
        let render_context = RenderContext::new(&device)?;
        let surface_data = device.surface().get_data(
            device.p_device(),
            Resolution {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            false,
        )?;
        let view = View::new(&device, &surface_data)?;

        let render_pipeline = device.create_graphics_pipeline(
            device.create_shader(include_bytes!("../shaders/spv/triangle.vert.spv"))?,
            device.create_shader(include_bytes!("../shaders/spv/triangle.frag.spv"))?,
            Default::default(),
        )?;

        let vertex_buffer = device.create_buffer_with_data(
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
        let index_buffer = device.create_buffer_with_data(
            &[0, 1, 2],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let init_time = Instant::now();

        Ok(Self {
            device,
            view,
            render_context,
            render_pipeline,
            surface_data,
            vertex_buffer,
            index_buffer,
            init_time,
        })
    }

    pub fn draw(&self) -> Result<bool> {
        let drawable = self.view.get_current_drawable(&self.device)?;

        self.render_context.begin(&self.device)?;
        {
            let surface_rect = Rect2D::from_width_height(
                self.surface_data.surface_resolution.width,
                self.surface_data.surface_resolution.height,
            );

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable, Default::default())],
                None,
            );
            {
                self.render_context
                    .bind_graphics_pipeline(&self.device, &self.render_pipeline);
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                self.render_context
                    .bind_index_buffer(&self.device, &self.index_buffer);
                self.render_context
                    .bind_vertex_buffer(&self.device, &self.vertex_buffer);

                // TODO: Maybe save a reference to the pipeline in `begin_rendering`, so that I don't need to pass it in here
                self.render_context.set_vertex_bytes(
                    &self.device,
                    &Mat4::rotate(
                        self.init_time.elapsed().as_secs_f32(),
                        Vec3::new(0.0, 0.0, 1.0),
                    ),
                    &self.render_pipeline.common,
                    0,
                )?;

                self.render_context.draw_offset(&self.device, 3, 0, 0);
            }
            self.render_context.end_rendering(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;

        self.view.present(&self.device, drawable)
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let renderer = Renderer::new(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        match virtual_keycode {
                            VirtualKeyCode::Escape => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        }
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::RedrawRequested(_) => {
                renderer.draw().unwrap();
            }
            _ => {}
        }

        window.request_redraw();
    });
}