mod app;
mod ui;

use app::App;
use camera::{Direction, MOVEMENT_DELTA, ROTATION_DELTA};
use cinder::{
    cinder::DefaultVertex,
    context::render_context::{AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment},
    resoruces::{
        buffer::{vk, BufferUsage},
        memory::{MemoryDescription, MemoryType},
        shader::ShaderStage,
    },
};
use egui_integration::egui;
use ember::GpuStagingBuffer;
use math::size::Size2D;
use std::time::Instant;
use tracing::Level;
use util::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const WINDOW_HEIGHT: u32 = 2000;
pub const WINDOW_WIDTH: u32 = 2000;

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
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let mut app = App::new(&event_loop, &window).unwrap();

    // TODO: Revisit this staging buffer idea, it could be good with some tweaks
    let mut staging_buffer = GpuStagingBuffer::new(
        &app.cinder,
        BufferUsage::empty().transfer_src(),
        MemoryDescription {
            ty: MemoryType::CpuVisible,
        },
    )
    .expect("Could not create GPU staging buffer");

    let mut vertex_buffer_offset = 0;
    let mut index_buffer_offset = 0;
    app.upload_context
        .begin(&app.cinder)
        .expect("Could not begin upload context");
    let mesh_draws = app
        .scene
        .meshes
        .iter()
        .map(|mesh| {
            let buffer_region = staging_buffer
                .copy_data(&mesh.indices)
                .expect("could not write to staging buffer");
            let index_buffer_size = size_of_slice(&mesh.indices);
            app.upload_context.copy_buffer(
                &app.cinder,
                staging_buffer.buffer(),
                &app.index_buffer,
                buffer_region.offset,
                (index_buffer_offset * std::mem::size_of::<u32>()) as u64,
                index_buffer_size,
            );

            let buffer_region = staging_buffer
                .copy_data(&mesh.vertices)
                .expect("could not write to staging buffer");
            let vertex_buffer_size = size_of_slice(&mesh.vertices);
            app.upload_context.copy_buffer(
                &app.cinder,
                staging_buffer.buffer(),
                &app.vertex_buffer,
                buffer_region.offset,
                (vertex_buffer_offset * std::mem::size_of::<DefaultVertex>()) as u64,
                vertex_buffer_size,
            );

            let num_indices = mesh.indices.len();
            let image_index = mesh.material_index.unwrap_or(0); //TODO: Actually handle this the right way, or use a white texture

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
    app.upload_context
        .end(
            &app.cinder,
            app.cinder.setup_fence(),
            app.cinder.present_queue(),
            &[],
            &[],
            &[],
        )
        .expect("could not end command context");

    let mut lock_movement = true;
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
                app.egui.on_event(&window_event);
                match window_event {
                    WindowEvent::Resized(size) => {
                        app.cinder
                            .resize(Size2D::new(size.width, size.height))
                            .expect("Could not resize device");
                        app.egui
                            .resize(&app.cinder)
                            .expect("Could not resize egui integration");
                        // TODO: This could be better
                        app.upload_context
                            .begin(&app.cinder)
                            .expect("could not begin upload context");
                        {
                            app.upload_context.transition_depth_image(&app.cinder);
                        }
                        app.upload_context
                            .end(
                                &app.cinder,
                                app.cinder.setup_fence(),
                                app.cinder.present_queue(),
                                &[],
                                &[],
                                &[],
                            )
                            .expect("could not end upload context");
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        app.keyboard_state.update(input);
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
                let (present_index, _is_suboptimal) = app
                    .cinder
                    .acquire_next_image()
                    .expect("Could not acquire swapchain image");
                // TODO: Handle is_suboptimal

                app.render_context
                    .begin(&app.cinder)
                    .expect("Could not begin graphics context");
                {
                    app.render_context.reset_query_pool(
                        app.cinder.device(),
                        &app.cinder.profiling.timestamp_query_pool,
                    );
                    app.render_context.write_timestamp(
                        app.cinder.device(),
                        &app.cinder.profiling.timestamp_query_pool,
                        0,
                    );

                    let delta_time = start.elapsed().as_secs_f32() / 2.0;
                    let color = [delta_time.sin(), 0.0, 0.0, 0.0];

                    let surface_rect = app.cinder.surface_rect();

                    app.render_context
                        .transition_undefined_to_color(&app.cinder, present_index);

                    app.render_context.begin_rendering(
                        &app.cinder,
                        surface_rect,
                        &[
                            RenderAttachment::color(app.cinder.swapchain(), present_index)
                                .load_op(AttachmentLoadOp::Clear)
                                .store_op(AttachmentStoreOp::Store)
                                .layout(Layout::ColorAttachment),
                        ],
                        Some(
                            RenderAttachment::depth(app.cinder.depth_image())
                                .load_op(AttachmentLoadOp::Clear)
                                .store_op(AttachmentStoreOp::DontCare)
                                .layout(Layout::DepthAttachment),
                        ),
                    );
                    {
                        app.render_context
                            .bind_graphics_pipeline(&app.cinder, &app.graphics_pipeline);
                        app.render_context
                            .bind_viewport(&app.cinder, surface_rect, true);
                        app.render_context.bind_scissor(&app.cinder, surface_rect);
                        app.render_context
                            .bind_index_buffer(&app.cinder, &app.index_buffer);
                        app.render_context.bind_descriptor_sets(
                            &app.cinder,
                            &app.graphics_pipeline,
                            &[app.bind_group.0],
                        );

                        for draw in &mesh_draws {
                            app.render_context
                                .push_constant(
                                    &app.cinder,
                                    &app.graphics_pipeline,
                                    ShaderStage::Vertex,
                                    0,
                                    util::as_u8_slice(&color),
                                )
                                .unwrap();

                            app.render_context
                                .push_constant(
                                    &app.cinder,
                                    &app.graphics_pipeline,
                                    ShaderStage::Fragment,
                                    0,
                                    util::as_u8_slice(&draw.image_index),
                                )
                                .unwrap();

                            app.render_context.draw_offset(
                                &app.cinder,
                                draw.num_indices as u32,
                                draw.index_buffer_offset,
                                draw.vertex_buffer_offset,
                            );
                        }
                    }
                    app.render_context.end_rendering(&app.cinder);

                    // Ui/egui render pass
                    app.egui
                        .run(
                            &app.cinder,
                            &app.upload_context,
                            &app.render_context,
                            present_index,
                            &window,
                            |egui_context| {
                                egui::TopBottomPanel::top("Cinder").show(egui_context, |ui| {
                                    app.cinder_ui.show_tabs(ui);
                                });

                                app.cinder_ui.show_selected_tab(egui_context, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("lock movement");
                                        ui.checkbox(&mut lock_movement, "toggle with `C`");
                                    });
                                    ui.collapsing("profiling", |ui| {
                                        ui.label(format!(
                                            "FPS: {}",
                                            (1e3 / frame_cpu_average).round() as u32
                                        ));
                                        ui.label(format!(
                                            "Average CPU: {:.5} ms",
                                            frame_cpu_average
                                        ));
                                        ui.label(format!(
                                            "Average GPU: {:.5} ms",
                                            frame_gpu_average
                                        ));
                                    });
                                    ui.collapsing("init", |ui| {
                                        ui.label(format!("total time: {} s", init_time));
                                        ui.label(format!("scene load: {} s", app.scene_load_time));
                                    });
                                    ui.collapsing("camera", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("movement speed: ");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut app.camera.movement_speed,
                                                )
                                                .speed(MOVEMENT_DELTA),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("rotation speed: ");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut app.camera.rotation_speed,
                                                )
                                                .speed(ROTATION_DELTA),
                                            );
                                        });
                                    });
                                })
                            },
                        )
                        .expect("Could not run egui");

                    app.render_context.write_timestamp(
                        app.cinder.device(),
                        &app.cinder.profiling.timestamp_query_pool,
                        1,
                    );

                    app.render_context
                        .transition_color_to_present(&app.cinder, present_index);
                }
                app.render_context
                    .end(
                        &app.cinder,
                        app.cinder.draw_fence(),
                        app.cinder.present_queue(),
                        &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                        &[app.cinder.present_semaphore()],
                        &[app.cinder.render_semaphore()],
                    )
                    .expect("Could not end graphics context");
                app.cinder
                    .present(present_index)
                    .expect("Could not submit graphics work");
            }
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    // TODO: Maybe using the mouse_state concept makes more sense
                    if !lock_movement {
                        app.camera.rotate(delta);
                    }
                }
                winit::event::DeviceEvent::MouseWheel { delta } => {
                    println!("{:?}", delta)
                }
                _ => {}
            },
            _ => {}
        }

        if !lock_movement {
            // TODO: Clean this up
            if app.keyboard_state.is_down(VirtualKeyCode::W) {
                app.camera.update_position(Direction::Front);
            }
            if app.keyboard_state.is_down(VirtualKeyCode::S) {
                app.camera.update_position(Direction::Back);
            }
            if app.keyboard_state.is_down(VirtualKeyCode::A) {
                app.camera.update_position(Direction::Left);
            }
            if app.keyboard_state.is_down(VirtualKeyCode::D) {
                app.camera.update_position(Direction::Right);
            }
            if app.keyboard_state.is_down(VirtualKeyCode::Space) {
                app.camera.update_position(Direction::Up);
            }
            if app.keyboard_state.is_down(VirtualKeyCode::LShift) {
                app.camera.update_position(Direction::Down);
            }

            let surface_size = app.cinder.surface_size();

            app.uniform_buffer
                .mem_copy(
                    0,
                    std::slice::from_ref(
                        &app.camera.get_matrices(
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

        let timestamp_results = app
            .cinder
            .device()
            .get_query_pool_results_u64(&app.cinder.profiling.timestamp_query_pool, 0, 2)
            .unwrap_or_else(|_| vec![]);
        if !timestamp_results.is_empty() {
            let gpu_begin = timestamp_results[0] as f32
                * app.cinder.device().properties().limits.timestamp_period
                * 1e-6;
            let gpu_end = timestamp_results[1] as f32
                * app.cinder.device().properties().limits.timestamp_period
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
