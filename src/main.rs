mod ui;

use crate::ui::Ui;
use camera::{Direction, PerspectiveData, MOVEMENT_DELTA, ROTATION_DELTA};
use cinder::{
    cinder::{Cinder, DefaultUniformBufferObject, DefaultVertex},
    context::{
        render_context::{
            AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment, RenderContextDescription,
        },
        upload_context::UploadContextDescription,
    },
    resoruces::{
        bind_group::{
            bindless_bind_group_flags, BindGroupBindInfo, BindGroupLayoutData, BindGroupSet,
            BindGroupType, BindGroupWriteBuilder, BindGroupWriteData, NewBindGroup,
            NewBindGroupLayout, NewBindGroupPool,
        },
        buffer::{vk, BufferDescription, BufferUsage},
        image::{Format, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{ColorBlendState, GraphicsPipelineDescription},
        shader::{ShaderDescription, ShaderStage},
    },
    InitData, Resolution,
};
use egui_integration::{egui, EguiIntegration};
use ember::GpuStagingBuffer;
use input::keyboard::KeyboardState;
use math::size::Size2D;
use std::{path::PathBuf, time::Instant};
use tracing::Level;
use util::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct MeshDraw {
    index_buffer_offset: u32,
    num_indices: usize,
    vertex_buffer_offset: i32,
    image_index: usize,
}

// TODO: verify that all triple buffering stuff is working

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
        vsync: true,
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
            bytes: include_bytes!("../shaders/spv/default.vert.spv"),
        })
        .expect("Could not create vertex shader");
    let fragment_shader = cinder
        .create_shader(ShaderDescription {
            bytes: include_bytes!("../shaders/spv/default.frag.spv"),
        })
        .expect("Could not create fragment shader");

    // Load model
    let scene_load_start = Instant::now();
    let (scene, image_buffers) = scene::ObjScene::load_or_achive(
        PathBuf::from("assets").join("models").join("sponza"),
        "sponza.obj",
    )
    .unwrap_or_else(|err| panic!("Could not load mesh: {}", err));
    let scene_load_time = scene_load_start.elapsed().as_secs_f32();

    let (num_vertices, num_indices) =
        scene.meshes.iter().fold((0, 0), |(n_vert, n_index), mesh| {
            (n_vert + mesh.vertices.len(), n_index + mesh.indices.len())
        });

    let index_buffer = cinder
        .create_buffer(BufferDescription {
            size: (num_indices * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsage::empty().index().transfer_dst(),
            memory_desc: MemoryDescription {
                ty: MemoryType::GpuOnly,
            },
        })
        .expect("Could not create index buffer");
    let vertex_buffer = cinder
        .create_buffer(BufferDescription {
            size: (num_vertices * std::mem::size_of::<DefaultVertex>()) as u64,
            usage: BufferUsage::empty().storage().transfer_dst(),
            memory_desc: MemoryDescription {
                ty: MemoryType::GpuOnly,
            },
        })
        .expect("Could not create vertex buffer");
    // TODO: Revisit this staging buffer idea, it could be good with some tweaks
    let mut staging_buffer = GpuStagingBuffer::new(
        &cinder,
        BufferUsage::empty().transfer_src(),
        MemoryDescription {
            ty: MemoryType::CpuVisible,
        },
    )
    .expect("Could not create GPU staging buffer");
    let mut vertex_buffer_offset = 0;
    let mut index_buffer_offset = 0;
    upload_context
        .begin(&cinder)
        .expect("Could not begin upload context");
    let mesh_draws = scene
        .meshes
        .iter()
        .map(|mesh| {
            let buffer_region = staging_buffer
                .copy_data(&mesh.indices)
                .expect("could not write to staging buffer");
            let index_buffer_size = size_of_slice(&mesh.indices);
            upload_context.copy_buffer(
                &cinder,
                staging_buffer.buffer(),
                &index_buffer,
                buffer_region.offset,
                (index_buffer_offset * std::mem::size_of::<u32>()) as u64,
                index_buffer_size,
            );

            let buffer_region = staging_buffer
                .copy_data(&mesh.vertices)
                .expect("could not write to staging buffer");
            let vertex_buffer_size = size_of_slice(&mesh.vertices);
            upload_context.copy_buffer(
                &cinder,
                staging_buffer.buffer(),
                &vertex_buffer,
                buffer_region.offset,
                (vertex_buffer_offset * std::mem::size_of::<DefaultVertex>()) as u64,
                vertex_buffer_size,
            );

            let num_indices = mesh.indices.len();
            let image_index = mesh.material_index.unwrap_or_else(|| 0); //TODO: Actually handle this the right way, or use a white texture

            let ret = MeshDraw {
                index_buffer_offset: index_buffer_offset as u32,
                num_indices,
                vertex_buffer_offset: vertex_buffer_offset as i32,
                image_index,
            };
            index_buffer_offset += mesh.indices.len();
            vertex_buffer_offset += mesh.vertices.len();
            ret
        })
        .collect::<Vec<_>>();
    upload_context
        .end(
            &cinder,
            cinder.setup_fence(),
            cinder.present_queue(),
            &[],
            &[],
            &[],
        )
        .expect("could not end command context");

    let mut camera = camera::Camera::from_data(PerspectiveData::default());

    // Create and upload uniform buffer
    let surface_size = cinder.surface_size();

    let uniform_buffer = cinder
        .create_buffer(BufferDescription {
            size: std::mem::size_of::<DefaultUniformBufferObject>() as u64,
            usage: BufferUsage::empty().uniform(),
            memory_desc: MemoryDescription {
                ty: MemoryType::CpuVisible,
            },
        })
        .expect("Could not create uniform buffer");
    uniform_buffer
        .mem_copy(
            0,
            std::slice::from_ref(
                &camera.get_matrices(surface_size.width() as f32, surface_size.height() as f32),
            ),
        )
        .expect("Could not write to uniform buffer");

    let sampler = cinder.create_sampler().expect("Could not create sampler");
    upload_context
        .begin(&cinder)
        .expect("could not begin upload context");
    // Create and upload image
    let (images, image_bind_infos): (Vec<_>, Vec<_>) = image_buffers
        .iter()
        .enumerate()
        .map(|(idx, image)| {
            let image_buffer = cinder
                .create_buffer(BufferDescription {
                    size: size_of_slice(&image.data),
                    usage: BufferUsage::empty().transfer_src(),
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })
                .expect("Could not create image buffer");
            image_buffer
                .mem_copy(0, &image.data)
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

            let info = texture.bind_info(&sampler, idx as u32);

            (
                texture,
                BindGroupBindInfo {
                    dst_binding: 2,
                    data: BindGroupWriteData::Image(info),
                },
            )
        })
        .unzip();
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

    // TODO: bind group layout stuff is bad here
    let vertex_buffer_info = vertex_buffer.bind_info();
    let uniform_buffer_info = uniform_buffer.bind_info();

    let new_pool = NewBindGroupPool::new(&cinder).unwrap();
    let new_layout = NewBindGroupLayout::new(
        &cinder,
        &[
            BindGroupLayoutData {
                binding: 0,
                ty: BindGroupType::UniformBuffer,
                count: 1,
                shader_stage: ShaderStage::Vertex,
                flags: Default::default(),
            },
            BindGroupLayoutData {
                binding: 1,
                ty: BindGroupType::StorageBuffer,
                count: 1,
                shader_stage: ShaderStage::Vertex,
                flags: Default::default(),
            },
            BindGroupLayoutData {
                binding: 2,
                ty: BindGroupType::ImageSampler,
                count: cinder.max_bindless_descriptor_count(),
                shader_stage: ShaderStage::Fragment,
                flags: bindless_bind_group_flags(),
            },
        ],
    )
    .unwrap();
    let new_set = NewBindGroup::new(&cinder, &new_pool, &new_layout).unwrap();

    new_set.write(
        &cinder,
        &[
            BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::Uniform(uniform_buffer_info),
            },
            BindGroupBindInfo {
                dst_binding: 1,
                data: BindGroupWriteData::Storage(vertex_buffer_info),
            },
        ],
    );
    new_set.write(&cinder, &image_bind_infos);

    let pipeline = cinder
        .create_graphics_pipeline(GraphicsPipelineDescription {
            vertex_shader,
            fragment_shader,
            blending: ColorBlendState::add(),
            depth_testing_enabled: true,
            backface_culling: true,
            uses_depth: true,
            bind_group_layout: Some(new_layout),
        })
        .expect("Could not create graphics pipeline");

    // Egui integration
    let mut cinder_ui = Ui::new();
    let mut egui = EguiIntegration::new(
        &event_loop,
        &mut cinder,
        cinder_ui.visuals(),
        cinder_ui.ui_scale(),
    )
    .expect("Could not create event loop");

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
                        cinder
                            .resize(Size2D::new(size.width, size.height))
                            .expect("Could not resize device");
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
                            .end(
                                &cinder,
                                cinder.setup_fence(),
                                cinder.present_queue(),
                                &[],
                                &[],
                                &[],
                            )
                            .expect("could not end upload context");
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
                    let color = [delta_time.sin(), 0.0, 0.0, 0.0];

                    let surface_rect = cinder.surface_rect();

                    render_context.transition_undefined_to_color(&cinder, present_index);

                    render_context.begin_rendering(
                        &cinder,
                        surface_rect,
                        &[RenderAttachment::color(cinder.swapchain(), present_index)
                            .load_op(AttachmentLoadOp::Clear)
                            .store_op(AttachmentStoreOp::Store)
                            .layout(Layout::ColorAttachment)],
                        Some(
                            RenderAttachment::depth(cinder.depth_image())
                                .load_op(AttachmentLoadOp::Clear)
                                .store_op(AttachmentStoreOp::DontCare)
                                .layout(Layout::DepthAttachment),
                        ),
                    );
                    {
                        render_context.bind_graphics_pipeline(&cinder, &pipeline);
                        render_context.bind_viewport(&cinder, surface_rect, true);
                        render_context.bind_scissor(&cinder, surface_rect);
                        render_context.bind_index_buffer(&cinder, &index_buffer);
                        render_context.bind_descriptor_sets(&cinder, &pipeline, &[new_set.0]);

                        for draw in &mesh_draws {
                            render_context
                                .push_constant(
                                    &cinder,
                                    &pipeline,
                                    ShaderStage::Vertex,
                                    0,
                                    util::as_u8_slice(&color),
                                )
                                .unwrap();

                            render_context
                                .push_constant(
                                    &cinder,
                                    &pipeline,
                                    ShaderStage::Fragment,
                                    0,
                                    util::as_u8_slice(&draw.image_index),
                                )
                                .unwrap();

                            render_context.draw_offset(
                                &cinder,
                                draw.num_indices as u32,
                                draw.index_buffer_offset,
                                draw.vertex_buffer_offset,
                            );
                        }
                    }
                    render_context.end_rendering(&cinder);

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
                    )
                    .expect("Could not run egui");

                    render_context.write_timestamp(
                        cinder.device(),
                        &cinder.profiling.timestamp_query_pool,
                        1,
                    );

                    render_context.transition_color_to_present(&cinder, present_index);
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
                    .present(present_index)
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

            uniform_buffer
                .mem_copy(
                    0,
                    std::slice::from_ref(
                        &camera.get_matrices(
                            surface_size.width() as f32,
                            surface_size.height() as f32,
                        ),
                    ),
                )
                .expect("Could not write to uniform buffer");
        }

        let frame_dt = frame_start.elapsed().as_millis() as f32;
        if frame_cpu_average == f32::MAX {
            frame_cpu_average = frame_dt;
        } else {
            frame_cpu_average = frame_cpu_average * 0.95 + frame_dt * 0.05;
        }

        let timestamp_results = cinder
            .device()
            .get_query_pool_results_u64(&cinder.profiling.timestamp_query_pool, 0, 2)
            .unwrap_or_else(|_| vec![]);
        if !timestamp_results.is_empty() {
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
        }

        window.request_redraw();
    });
}
