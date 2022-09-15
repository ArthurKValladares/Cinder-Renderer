use std::path::Path;

use cinder::{
    context::{graphics_context::GraphicsContextDescription, Context},
    device::Device,
    resoruces::{
        pipeline::GraphicsPipelineDescription,
        render_pass::{self, RenderPassAttachmentDesc, RenderPassDescription},
        shader::{ShaderDescription, ShaderStage},
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
    let graphics_context = device
        .create_graphics_context(GraphicsContextDescription {})
        .expect("Could not create graphics context");
    let vertex_shader = device
        .create_shader(ShaderDescription {
            stage: ShaderStage::Vertex,
            path: Path::new("shaders/default.vert"),
        })
        .expect("Could not create vertex shader");
    let fragment_shader = device
        .create_shader(ShaderDescription {
            stage: ShaderStage::Fragment,
            path: Path::new("shaders/default.frag"),
        })
        .expect("Could not create fragment shader");
    let render_pass = device
        .create_render_pass(RenderPassDescription {
            color_attachments: [
                RenderPassAttachmentDesc::with_format(device.surface_format()).clear_input(),
            ],
        })
        .expect("Could not create render pass");
    let pipeline = device
        .create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            render_pass,
        })
        .expect("Could not create graphics pipeline");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {}
            Event::RedrawRequested(_) => {
                graphics_context
                    .begin(&device)
                    .expect("Could not begin graphics context");
                graphics_context.set_graphics_pipeline(&pipeline);
                graphics_context.draw();
                graphics_context
                    .end(&device)
                    .expect("Could not end graphics context");

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
