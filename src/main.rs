use std::path::Path;

use cinder::{
    context::{graphics_context::GraphicsContextDescription, Context},
    device::Device,
    resoruces::{
        pipeline::PipelineDescription,
        render_pass::{self, RenderPassDescription},
        shader::ShaderDescription,
    },
    InitData, Resolution,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    const WINDOW_HEIGHT: u32 = 1000;
    const WINDOW_WIDTH: u32 = 1000;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let init_data = InitData {
        backbuffer_resolution: Resolution {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        },
    };
    let device = Device::new(&window, init_data).expect("could not create cinder device");
    let graphics_context = device.create_graphics_context(GraphicsContextDescription {});
    let vertex_shader = device.create_shader(ShaderDescription {
        path: Path::new("shaders/default.vert"),
    });
    let fragment_shader = device.create_shader(ShaderDescription {
        path: Path::new("shaders/default.frag"),
    });
    let render_pass = device.create_render_pass(RenderPassDescription {});
    let pipeline = device.create_pipeline(PipelineDescription {});

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {}
            Event::RedrawRequested(_) => {
                graphics_context.begin();
                graphics_context.set_pipeline(&pipeline);
                graphics_context.draw();
                graphics_context.end();

                device.submit_work(&graphics_context);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
