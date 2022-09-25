use std::{path::Path, time::Instant};

use cgmath::{Deg, Matrix4, Point3, Vector3};
use cinder::{
    context::{
        render_context::RenderContextDescription,
        upload_context::{self, UploadContextDescription},
    },
    device::{Device, Vertex},
    resoruces::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        memory::{MemoryDescription, MemoryType},
        pipeline::GraphicsPipelineDescription,
        render_pass::{self, RenderPassAttachmentDesc, RenderPassDescription},
        shader::{ShaderDescription, ShaderStage},
        texture::{Format, TextureDescription},
    },
    InitData, Resolution,
};
use math::size::Size2D;
use tracing::Level;
use util::*;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct UniformBufferObject {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
}

fn update_uniform_buffer(
    device: &Device,
    uniform_buffer_data: &mut UniformBufferObject,
    uniform_buffer: &Buffer,
    delta_time: f32,
) {
    uniform_buffer_data.model =
        Matrix4::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), Deg(90.0) * delta_time);

    device.copy_data_to_buffer(uniform_buffer, std::slice::from_ref(uniform_buffer_data));
}

fn main() {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(collector);

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
                RenderPassAttachmentDesc::clear_store(device.surface_format()).clear_input(),
            ],
            depth_attachment: Some(RenderPassAttachmentDesc::clear_dont_care(Format::D32SFloat)),
        })
        .expect("Could not create render pass");
    let pipeline = device
        .create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            render_pass: &render_pass,
        })
        .expect("Could not create graphics pipeline");

    // Load model
    let mut meshes =
        scene::Mesh::from_obj_path("./assets/models/viking_room.obj").expect("Could not load mesh");
    let mesh = meshes.remove(0);

    // Create and bind index buffer
    let index_buffer = device
        .create_buffer(BufferDescription {
            size: size_of_slice(&mesh.indices),
            usage: BufferUsage::Index,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create index buffer");
    device
        .copy_data_to_buffer(&index_buffer, &mesh.indices)
        .expect("Could not write to index buffer");
    device
        .bind_buffer(&index_buffer)
        .expect("Could not bind index buffer");

    // Create and bind vertex buffer
    let vertex_buffer = device
        .create_buffer(BufferDescription {
            size: size_of_slice(&mesh.vertices),
            usage: BufferUsage::Vertex,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create vertex buffer");
    device
        .copy_data_to_buffer(&vertex_buffer, &mesh.vertices)
        .expect("Could not write to vertex buffer");
    device
        .bind_buffer(&vertex_buffer)
        .expect("Could not bind vertex buffer");

    // Create and upload uniform buffer
    let surface_size = device.surface_size();
    let mut uniform_data = UniformBufferObject {
        model: Matrix4::from_angle_z(Deg(90.0)),
        view: Matrix4::look_at(
            Point3::new(2.0, 2.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ),
        proj: {
            let mut proj = cgmath::perspective(
                Deg(45.0),
                surface_size.width() as f32 / surface_size.height() as f32,
                0.1,
                10.0,
            );
            proj[1][1] = proj[1][1] * -1.0;
            proj
        },
    };
    let uniform_buffer = device
        .create_buffer(BufferDescription {
            size: std::mem::size_of::<UniformBufferObject>() as u64,
            usage: BufferUsage::Uniform,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create vertex buffer");
    device
        .copy_data_to_buffer(&uniform_buffer, std::slice::from_ref(&uniform_data))
        .expect("Could not write to vertex buffer");
    device
        .bind_buffer(&uniform_buffer)
        .expect("Could not bind vertex buffer");
    // Create and upload image
    let image =
        image::load_from_memory(include_bytes!("../assets/textures/viking_room.png")).unwrap();
    let image = image.flipv();
    let image = image.to_rgba8();

    let (image_width, image_height) = image.dimensions();
    let image_data = image.into_raw();

    let image_buffer = device
        .create_buffer(BufferDescription {
            size: size_of_slice(&image_data),
            usage: BufferUsage::TransferSrc,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create image buffer");
    device
        .copy_data_to_buffer(&image_buffer, &image_data)
        .expect("Could not write to image buffer");
    device
        .bind_buffer(&image_buffer)
        .expect("Could not bind image buffer");

    let ferris_texture = device
        .create_texture(TextureDescription {
            format: Format::R8G8B8A8Unorm,
            size: Size2D::new(image_width, image_height),
        })
        .expect("could not create texture");

    upload_context
        .begin(&device)
        .expect("could not begin upload context");
    {
        upload_context.transition_depth_image(&device);
        upload_context.texture_barrier_start(&device, &ferris_texture);
        upload_context.copy_buffer_to_texture(&device, &image_buffer, &ferris_texture);
        upload_context.texture_barrier_end(&device, &ferris_texture);
    }
    upload_context
        .end(&device)
        .expect("could not end upload context");
    device
        .submit_upload_work(&upload_context)
        .expect("could not submit upload work");

    let sampler = device.create_sampler().expect("Could not create sampler");
    device.update_descriptor_set(&ferris_texture, &sampler, &uniform_buffer);

    let start = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
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
                    let delta_time = start.elapsed().as_secs_f32() / 2.0;
                    update_uniform_buffer(&device, &mut uniform_data, &uniform_buffer, delta_time);

                    render_context.begin_render_pass(&device, &render_pass, present_index);
                    {
                        let surface_rect = device.surface_rect();

                        render_context.bind_graphics_pipeline(&device, &pipeline);
                        render_context.bind_descriptor_sets(&device, &pipeline);
                        render_context.bind_vertex_buffer(&device, &vertex_buffer);
                        render_context.bind_index_buffer(&device, &index_buffer);
                        render_context.bind_viewport(&device, surface_rect);
                        render_context.bind_scissor(&device, surface_rect);
                        render_context.draw(&device, mesh.indices.len() as u32);
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
