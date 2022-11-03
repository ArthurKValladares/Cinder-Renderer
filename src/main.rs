mod ui;

use camera::{
    CameraMatrices, CameraType, Direction, PerspectiveData, MOVEMENT_DELTA, ROTATION_DELTA,
};
use cgmath::{Deg, Matrix4, Point3, Vector3};
use cinder::{
    cinder::{Cinder, Vertex},
    context::{render_context::RenderContextDescription, upload_context::UploadContextDescription},
    resoruces::{
        bind_group::{BindGroupLayoutBuilder, BindGroupSetBuilder, BindGroupType},
        buffer::{vk, Buffer, BufferDescription, BufferUsage},
        image::{Format, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{
            push_constant::PushConstant, GraphicsPipelineDescription, VertexAttributeDesc,
            VertexInputStateDesc,
        },
        render_pass::{
            self, AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderPassAttachmentDesc,
            RenderPassDescription,
        },
        shader::{ShaderDescription, ShaderStage},
    },
    InitData, Resolution,
};
use egui_integration::{egui, EguiIntegration};
use input::keyboard::KeyboardState;
use math::size::Size2D;
use render_pass::Layout;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use tracing::Level;
use util::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::ui::Ui;

struct MeshDraw {
    index_buffer: Buffer,
    num_indices: usize,
    vertex_buffer: Buffer,
    image_index: usize,
}

// TODO: verify that all triple buffering stuff is working

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct ModelPushConstant {
    mat: Matrix4<f32>,
}

fn update_model_push_constant(model: &mut ModelPushConstant, delta_time: f32) {
    model.mat = Matrix4::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), Deg(90.0) * delta_time);
}

fn main() {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(collector)
        .expect("Could not set tracing global subscriber");

    let init_start = Instant::now();
    const WINDOW_HEIGHT: u32 = 2000;
    const WINDOW_WIDTH: u32 = 2000;

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
    let mut cinder = Cinder::new(&window, init_data).expect("could not create cinder device");
    let render_context = cinder
        .create_render_context(RenderContextDescription {})
        .expect("Could not create graphics context");
    let upload_context = cinder
        .create_upload_context(UploadContextDescription {})
        .expect("could not create upload context");

    let vertex_shader = cinder
        .create_shader(ShaderDescription {
            stage: ShaderStage::Vertex,
            path: Path::new("shaders/spv/default.vert.spv"),
        })
        .expect("Could not create vertex shader");
    let fragment_shader = cinder
        .create_shader(ShaderDescription {
            stage: ShaderStage::Fragment,
            path: Path::new("shaders/spv/default.frag.spv"),
        })
        .expect("Could not create fragment shader");
    let mut render_pass = cinder
        .create_render_pass(RenderPassDescription {
            color_attachment: RenderPassAttachmentDesc::new(cinder.surface_format())
                .load_op(AttachmentLoadOp::Clear)
                .store_op(AttachmentStoreOp::Store)
                .final_layout(Layout::ColorAttachment),
            depth_attachment: Some(
                RenderPassAttachmentDesc::new(Format::D32_SFloat)
                    .load_op(AttachmentLoadOp::Clear)
                    .store_op(AttachmentStoreOp::Store)
                    .store_op(AttachmentStoreOp::Store)
                    .initial_layout(Layout::DepthAttachment)
                    .final_layout(Layout::DepthAttachment),
            ),
        })
        .expect("Could not create render pass");

    // Load model
    let scene_load_start = Instant::now();
    let (mut scene, image_buffers) = scene::ObjScene::load_or_achive(
        PathBuf::from("assets").join("models").join("sibenik"),
        "sibenik.obj",
    )
    .unwrap_or_else(|err| panic!("Could not load mesh: {}", err));
    let scene_load_time = scene_load_start.elapsed().as_secs_f32();

    // Create and bind index buffer
    let mesh_draws = scene
        .meshes
        .iter()
        .map(|mesh| {
            let index_buffer = cinder
                .create_buffer(BufferDescription {
                    size: size_of_slice(&mesh.indices),
                    usage: BufferUsage::Index,
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })
                .expect("Could not create index buffer");
            index_buffer
                .mem_copy(&mesh.indices)
                .expect("Could not write to index buffer");

            // Create and bind vertex buffer
            let vertex_buffer = cinder
                .create_buffer(BufferDescription {
                    size: size_of_slice(&mesh.vertices),
                    usage: BufferUsage::Vertex,
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })
                .expect("Could not create vertex buffer");
            vertex_buffer
                .mem_copy(&mesh.vertices)
                .expect("Could not write to vertex buffer");

            let num_indices = mesh.indices.len();
            let image_index = mesh.material_index.unwrap_or_else(|| 0); //TODO: Actually handle this the right way, or use a white texture

            MeshDraw {
                index_buffer,
                num_indices,
                vertex_buffer,
                image_index,
            }
        })
        .collect::<Vec<_>>();

    let mut camera = camera::Camera::from_type(CameraType::Perspective(PerspectiveData::default()));

    // Create and upload uniform buffer
    let surface_size = cinder.surface_size();
    let mut camera_matrices =
        camera.get_matrices(surface_size.width() as f32, surface_size.height() as f32);

    let uniform_buffer = cinder
        .create_buffer(BufferDescription {
            size: std::mem::size_of::<CameraMatrices>() as u64,
            usage: BufferUsage::Uniform,
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create uniform buffer");
    uniform_buffer
        .mem_copy(std::slice::from_ref(&camera_matrices))
        .expect("Could not write to uniform buffer");

    upload_context
        .begin(&cinder)
        .expect("could not begin upload context");
    // Create and upload image
    let images = image_buffers
        .iter()
        .map(|image| {
            let image_buffer = cinder
                .create_buffer(BufferDescription {
                    size: size_of_slice(&image.data),
                    usage: BufferUsage::TransferSrc,
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })
                .expect("Could not create image buffer");
            image_buffer
                .mem_copy(&image.data)
                .expect("Could not write to image buffer");

            let texture = cinder
                .create_image(ImageDescription {
                    format: Format::R8_G8_B8_A8_Unorm,
                    usage: Usage::Texture,
                    size: Size2D::new(image.width, image.height),
                })
                .expect("could not create texture");

            upload_context.image_barrier_start(&cinder, &texture);
            upload_context.copy_buffer_to_image(&cinder, &image_buffer, &texture);
            upload_context.image_barrier_end(&cinder, &texture);

            texture
        })
        .collect::<Vec<_>>();
    upload_context.transition_depth_image(&cinder);
    upload_context
        .end(
            &cinder,
            cinder.setup_fence(),
            cinder.present_queue(),
            &[],
            &[],
            &[],
        )
        .expect("could not end upload context");

    let sampler = cinder.create_sampler().expect("Could not create sampler");

    let buffer_info = uniform_buffer.bind_info();
    let bind_group_layout = BindGroupLayoutBuilder::default()
        .bind_buffer(0, BindGroupType::UniformBuffer, ShaderStage::Vertex)
        .bind_image(1, BindGroupType::ImageSampler, ShaderStage::Fragment)
        .build(&mut cinder)
        .expect("Could not create BindGroup");
    let bind_group_sets = images
        .iter()
        .map(|image| {
            let image_info = image.bind_info(&sampler);

            BindGroupSetBuilder::default()
                .bind_buffer(0, &buffer_info, BindGroupType::UniformBuffer)
                .bind_image(1, &image_info, BindGroupType::ImageSampler)
                .build_and_update(&mut cinder, &bind_group_layout)
                .expect("Could not create bind group set")
        })
        .collect::<Vec<_>>();
    let model_push_constant = PushConstant {
        stage: ShaderStage::Vertex,
        offset: 0,
        size: std::mem::size_of::<ModelPushConstant>() as u32,
    };
    let pipeline = cinder
        .create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            vertex_state: VertexInputStateDesc {
                binding: 0,
                stride: std::mem::size_of::<Vertex>() as u32,
                attributes: vec![
                    VertexAttributeDesc {
                        format: Format::R32_G32_B32_SFloat,
                        offset: offset_of!(Vertex, pos) as u32,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: offset_of!(Vertex, uv) as u32,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_B32_A32_SFloat,
                        offset: offset_of!(Vertex, color) as u32,
                    },
                ],
            },
            render_pass: &render_pass,
            desc_set_layouts: vec![bind_group_layout.layout],
            push_constants: vec![&model_push_constant],
            depth_testing_enabled: true,
            backface_culling: true,
        })
        .expect("Could not create graphics pipeline");

    let mut color = ModelPushConstant {
        mat: Matrix4::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), Deg(0.0)),
    };

    // Egui integration
    let mut cinder_ui = Ui::new();
    let mut egui = EguiIntegration::new(&event_loop, &mut cinder, cinder_ui.visuals())
        .expect("Could not create event loop");

    // TODO: need this `is_init` hack until winit fiexes their long-standing resize on startup bug :(
    let mut is_init = false;
    let mut lock_movement = true;
    let mut keyboard_state = KeyboardState::default();
    let init_time = init_start.elapsed().as_secs_f32();
    let start = Instant::now();
    let mut frame_cpu_average = f32::MAX;
    let mut frame_gpu_average = f32::MAX;
    event_loop.run(move |event, _, control_flow| {
        let frame_start = Instant::now();

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => {
                egui.on_event(&window_event);
                match window_event {
                    WindowEvent::Resized(size) => {
                        /*
                        if is_init {
                            return;
                        }

                        cinder
                            .resize(Size2D::new(size.width, size.height))
                            .expect("Could not resize device");
                        // TODO: easier way to re-create render passes
                        cinder.clean_render_pass(&mut render_pass);
                        render_pass = cinder
                            .create_render_pass(RenderPassDescription {
                                color_attachment: RenderPassAttachmentDesc::new(
                                    cinder.surface_format(),
                                )
                                .with_color_depth_ops(AttachmentOps {
                                    load: AttachmentLoadOp::Clear,
                                    store: AttachmentStoreOp::Store,
                                })
                                .with_layout_transition(LayoutTransition {
                                    initial_layout: Layout::Undefined,
                                    final_layout: Layout::ColorAttachment,
                                }),
                                depth_attachment: Some(
                                    RenderPassAttachmentDesc::new(Format::D32_SFloat)
                                        .with_color_depth_ops(AttachmentOps {
                                            load: AttachmentLoadOp::Clear,
                                            store: AttachmentStoreOp::Store,
                                        })
                                        .with_layout_transition(LayoutTransition {
                                            initial_layout: Layout::DepthAttachment,
                                            final_layout: Layout::DepthAttachment,
                                        }),
                                ),
                            })
                            .expect("Could not create render pass");
                        egui.resize(&cinder)
                            .expect("Could not resize egui integration");
                        // TODO: This could be better
                        upload_context
                            .begin(&cinder)
                            .expect("could not begin upload context");
                        {
                            upload_context.transition_depth_image(&cinder);
                        }
                        upload_context
                            .end(&cinder)
                            .expect("could not end upload context");
                        cinder
                            .submit_upload_work(&upload_context)
                            .expect("could not submit upload work");
                        */
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        keyboard_state.update(input);
                        if let Some(virtual_keycode) = input.virtual_keycode {
                            match virtual_keycode {
                                VirtualKeyCode::Escape => {
                                    *control_flow = ControlFlow::Exit;
                                }
                                VirtualKeyCode::C => {
                                    if input.state == ElementState::Pressed {
                                        // TODO: Visual representation of this
                                        lock_movement = !lock_movement;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                let (present_index, _is_suboptimal) = cinder
                    .acquire_next_image()
                    .expect("Could not acquire swapchain image");
                // TODO: Handle is_suboptimal

                render_context
                    .begin(&cinder)
                    .expect("Could not begin graphics context");
                {
                    render_context
                        .reset_query_pool(cinder.device(), &cinder.profiling.timestamp_query_pool);
                    render_context.write_timestamp(
                        cinder.device(),
                        &cinder.profiling.timestamp_query_pool,
                        0,
                    );

                    let delta_time = start.elapsed().as_secs_f32() / 2.0;
                    update_model_push_constant(&mut color, delta_time);

                    let surface_rect = cinder.surface_rect();

                    // Main render pass
                    render_context.begin_render_pass(
                        &cinder,
                        &render_pass,
                        present_index,
                        surface_rect,
                        &[
                            ClearValue::color([1.0, 0.0, 1.0, 1.0]),
                            ClearValue::depth(1.0, 0),
                        ],
                    );
                    {
                        render_context.bind_graphics_pipeline(&cinder, &pipeline);
                        render_context.bind_viewport(&cinder, surface_rect, true);
                        render_context.bind_scissor(&cinder, surface_rect);

                        for draw in &mesh_draws {
                            render_context.bind_descriptor_sets(
                                &cinder,
                                &pipeline,
                                &[bind_group_sets[draw.image_index].set],
                            );

                            render_context.bind_vertex_buffer(&cinder, &draw.vertex_buffer);
                            render_context.bind_index_buffer(&cinder, &draw.index_buffer);
                            render_context.push_constant(
                                &cinder,
                                &pipeline,
                                &model_push_constant,
                                util::as_u8_slice(&color),
                            );
                            render_context.draw(&cinder, draw.num_indices as u32);
                        }
                    }
                    render_context.end_render_pass(&cinder);

                    // Ui/egui render pass
                    egui.run(
                        &cinder,
                        &upload_context,
                        &render_context,
                        present_index,
                        &window,
                        |egui_context| {
                            egui::TopBottomPanel::top("Cinder").show(egui_context, |ui| {
                                cinder_ui.show_tabs(ui);
                            });

                            cinder_ui.show_selected_tab(egui_context, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("lock movement");
                                    ui.checkbox(&mut lock_movement, "toggle with `C`");
                                });
                                ui.collapsing("profiling", |ui| {
                                    ui.label(format!(
                                        "FPS: {}",
                                        (1e3 / frame_cpu_average).round() as u32
                                    ));
                                    ui.label(format!("Average CPU: {:.5} ms", frame_cpu_average));
                                    ui.label(format!("Average GPU: {:.5} ms", frame_gpu_average));
                                });
                                ui.collapsing("init", |ui| {
                                    ui.label(format!("total time: {} s", init_time));
                                    ui.label(format!("scene load: {} s", scene_load_time));
                                });
                                ui.collapsing("camera", |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("movement speed: ");
                                        ui.add(
                                            egui::DragValue::new(&mut camera.movement_speed)
                                                .speed(MOVEMENT_DELTA),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("rotation speed: ");
                                        ui.add(
                                            egui::DragValue::new(&mut camera.rotation_speed)
                                                .speed(ROTATION_DELTA),
                                        );
                                    });
                                });
                            })
                        },
                    );

                    render_context.write_timestamp(
                        cinder.device(),
                        &cinder.profiling.timestamp_query_pool,
                        1,
                    );
                }
                render_context
                    .end(
                        &cinder,
                        cinder.draw_fence(),
                        cinder.present_queue(),
                        &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                        &[cinder.present_semaphore()],
                        &[cinder.render_semaphore()],
                    )
                    .expect("Could not end graphics context");
                cinder
                    .present(&render_context, present_index)
                    .expect("Could not submit graphics work");
            }
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    // TODO: Maybe using the mouse_state concept makes more sense
                    if !lock_movement {
                        camera.rotate(delta);
                    }
                }
                _ => {}
            },
            Event::NewEvents(cause) => {
                if cause == StartCause::Init {
                    is_init = true;
                } else {
                    is_init = false;
                }
            }
            _ => {}
        }

        if !lock_movement {
            // TODO: Clean this up
            if keyboard_state.is_down(VirtualKeyCode::W) {
                camera.update_position(Direction::Front);
            }
            if keyboard_state.is_down(VirtualKeyCode::S) {
                camera.update_position(Direction::Back);
            }
            if keyboard_state.is_down(VirtualKeyCode::A) {
                camera.update_position(Direction::Left);
            }
            if keyboard_state.is_down(VirtualKeyCode::D) {
                camera.update_position(Direction::Right);
            }
            if keyboard_state.is_down(VirtualKeyCode::Space) {
                camera.update_position(Direction::Up);
            }
            if keyboard_state.is_down(VirtualKeyCode::LShift) {
                camera.update_position(Direction::Down);
            }

            let surface_size = cinder.surface_size();
            camera_matrices =
                camera.get_matrices(surface_size.width() as f32, surface_size.height() as f32);

            uniform_buffer
                .mem_copy(std::slice::from_ref(&camera_matrices))
                .expect("Could not write to uniform buffer");
        }

        let frame_dt = frame_start.elapsed().as_millis() as f32;
        if frame_cpu_average == f32::MAX {
            frame_cpu_average = frame_dt;
        } else {
            frame_cpu_average = frame_cpu_average * 0.95 + frame_dt * 0.05;
        }

        // TODO: Should not be a crash on fail
        let timestamp_results = cinder
            .device()
            .get_query_pool_results_u64(&cinder.profiling.timestamp_query_pool, 0, 2)
            .expect("Could not get query pool results");
        let gpu_begin = timestamp_results[0] as f32
            * cinder.device().properties().limits.timestamp_period
            * 1e-6;
        let gpu_end = timestamp_results[1] as f32
            * cinder.device().properties().limits.timestamp_period
            * 1e-6;
        let gpu_dt = gpu_end - gpu_begin;
        if frame_gpu_average == f32::MAX {
            frame_gpu_average = gpu_dt;
        } else {
            frame_gpu_average = frame_gpu_average * 0.95 + gpu_dt * 0.05;
        }

        window.request_redraw();
    });
}
