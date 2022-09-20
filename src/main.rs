use std::path::Path;

use cinder::{
    context::{
        render_context::RenderContextDescription,
        upload_context::{self, UploadContextDescription},
    },
    device::{Device, Vertex},
    resoruces::{
        buffer::{BufferDescription, BufferUsage},
        memory::{MemoryDescription, MemoryType},
        pipeline::GraphicsPipelineDescription,
        render_pass::{self, RenderPassAttachmentDesc, RenderPassDescription},
        shader::{ShaderDescription, ShaderStage},
        texture::{Format, TextureDescription},
    },
    InitData, Resolution,
};
use math::size::Size2D;
use util::*;
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
    let render_context = device
        .create_render_context(RenderContextDescription {})
        .expect("Could not create graphics context");
    let upload_context = device
        .create_upload_context(UploadContextDescription {})
        .expect("could not create upload context");
    let vertex_shader = device
        .create_shader(ShaderDescription {
            stage: ShaderStage::Vertex,
            path: Path::new("shaders/spv/default.vert.spv"),
        })
        .expect("Could not create vertex shader");
    let fragment_shader = device
        .create_shader(ShaderDescription {
            stage: ShaderStage::Fragment,
            path: Path::new("shaders/spv/default.frag.spv"),
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
            render_pass: &render_pass,
        })
        .expect("Could not create graphics pipeline");

    // Create and bind index buffer
    let indices = [0u32, 1, 2];
    let index_buffer = device
        .create_buffer(BufferDescription {
            size: size_of_slice(&indices),
            usage: BufferUsage::Index,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create index buffer");
    device
        .copy_data_to_buffer(&index_buffer, &indices)
        .expect("Could not write to index buffer");
    device
        .bind_buffer(&index_buffer)
        .expect("Could not bind index buffer");

    // Create and bind vertex buffer
    let vertices = [
        Vertex {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
        Vertex {
            pos: [0.0, -1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ];
    let vertex_buffer = device
        .create_buffer(BufferDescription {
            size: size_of_slice(&vertices),
            usage: BufferUsage::Vertex,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create vertex buffer");
    device
        .copy_data_to_buffer(&vertex_buffer, &vertices)
        .expect("Could not write to vertex buffer");
    device
        .bind_buffer(&vertex_buffer)
        .expect("Could not bind vertex buffer");

    // Create and upload image
    let image = image::load_from_memory(include_bytes!("../assets/textures/ferris.png"))
        .unwrap()
        .to_rgba8();
    let (image_width, image_height) = image.dimensions();
    let image_data = image.into_raw();

    let ferris_image = device
        .create_texture(TextureDescription {
            format: Format::R8G8B8A8Unorm,
            size: Size2D::new(image_width, image_height),
        })
        .expect("could not create texture");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {}
            Event::RedrawRequested(_) => {
                let present_index = device
                    .acquire_next_image()
                    .expect("Could not acquire swapchain image");

                render_context
                    .begin(&device)
                    .expect("Could not begin graphics context");
                {
                    render_context.begin_render_pass(&device, &render_pass, present_index);
                    {
                        render_context.set_graphics_pipeline(&device, &pipeline);
                        render_context.set_vertex_buffer(&device, &vertex_buffer);
                        render_context.set_index_buffer(&device, &index_buffer);
                        let surface_rect = device.surface_rect();
                        render_context.set_viewport(&device, surface_rect);
                        render_context.set_scissor(&device, surface_rect);
                        render_context.draw(&device, &index_buffer, indices.len() as u32);
                    }
                    render_context.end_render_pass(&device, &render_pass);
                }
                render_context
                    .end(&device)
                    .expect("Could not end graphics context");

                device
                    .submit_graphics_work(&render_context, present_index)
                    .expect("Could not submit graphics work");
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
