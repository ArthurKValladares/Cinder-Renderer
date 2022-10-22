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
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{
            push_constant::PushConstant, GraphicsPipelineDescription, VertexAttributeDesc,
            VertexInputStateDesc,
        },
        render_pass::{self, RenderPassAttachmentDesc, RenderPassDescription},
        shader::{ShaderDescription, ShaderStage},
    },
    InitData, Resolution,
};
use egui_integration::{egui, EguiIntegration};
use input::keyboard::KeyboardState;
use math::size::Size2D;
use render_pass::{Layout, LayoutTransition};
use std::{path::Path, time::Instant};
use tracing::Level;
use util::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::ui::Ui;

struct MeshDraw {
    index_buffer: Buffer,
    num_indices: usize,
    vertex_buffer: Buffer,
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
            color_attachments: [
                RenderPassAttachmentDesc::clear_store(cinder.surface_format())
                    .with_layout_transition(LayoutTransition {
                        initial_layout: Layout::Undefined,
                        final_layout: Layout::ColorAttachment,
                    }),
            ],
            depth_attachment: Some(
                RenderPassAttachmentDesc::clear_store(Format::D32_SFloat).with_layout_transition(
                    LayoutTransition {
                        initial_layout: Layout::Undefined,
                        final_layout: Layout::DepthAttachment,
                    },
                ),
            ),
        })
        .expect("Could not create render pass");

    // Load model
    let scene_load_start = Instant::now();
    let mut scene = scene::ObjScene::load_or_achive("./assets/models/sponza", "sponza.obj")
        .expect("Could not load mesh");
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

            MeshDraw {
                index_buffer,
                num_indices,
                vertex_buffer,
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
    let images = scene
        .materials()
        .iter()
        .map(|material| {
            let path = scene.root().join(&material.diffuse_texture);
            let image =
                image::open(&path).expect(&format!("could not find image path: {:?}", path));
            let image = image.flipv();
            let image = image.to_rgba8();

            let (image_width, image_height) = image.dimensions();
            let image_data = image.into_raw();

            let image_buffer = cinder
                .create_buffer(BufferDescription {
                    size: size_of_slice(&image_data),
                    usage: BufferUsage::TransferSrc,
                    memory_desc: MemoryDescription {
                        ty: MemoryType::CpuVisible,
                    },
                })
                .expect("Could not create image buffer");
            image_buffer
                .mem_copy(&image_data)
                .expect("Could not write to image buffer");

            let texture = cinder
                .create_image(ImageDescription {
                    format: Format::R8_G8_B8_A8_Unorm,
                    usage: Usage::Texture,
                    size: Size2D::new(image_width, image_height),
                })
                .expect("could not create texture");

            upload_context.image_barrier_start(&cinder, &texture);
            upload_context.copy_buffer_to_image(&cinder, &image_buffer, &texture);
            upload_context.image_barrier_end(&cinder, &texture);

            texture
        })
        .collect::<Vec<_>>();
    upload_context
        .end(&cinder)
        .expect("could not end upload context");
    cinder
        .submit_upload_work(&upload_context)
        .expect("could not submit upload work");

    let sampler = cinder.create_sampler().expect("Could not create sampler");

    let buffer_info = uniform_buffer.bind_info();
    let image_info = images[0].bind_info(&sampler);
    let bind_group_layout = BindGroupLayoutBuilder::default()
        .bind_buffer(0, BindGroupType::UniformBuffer, ShaderStage::Vertex)
        .bind_image(1, BindGroupType::ImageSampler, ShaderStage::Fragment)
        .build(&mut cinder)
        .expect("Could not create BindGroup");
    let bind_group_set = BindGroupSetBuilder::default()
        .bind_buffer(0, &buffer_info, BindGroupType::UniformBuffer)
        .bind_image(1, &image_info, BindGroupType::ImageSampler)
        .build_and_update(&mut cinder, &bind_group_layout)
        .expect("Could not create bind group set");
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
                        format: Format::R32_G32_B32_A32_SFloat,
                        offset: offset_of!(Vertex, pos) as u32,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_B32_A32_SFloat,
                        offset: offset_of!(Vertex, color) as u32,
                    },
                    VertexAttributeDesc {
                        format: Format::R32_G32_SFloat,
                        offset: offset_of!(Vertex, uv) as u32,
                    },
                ],
            },
            render_pass: &render_pass,
            desc_set_layouts: vec![bind_group_layout.layout],
            push_constants: vec![&model_push_constant],
            depth_testing_enabled: true,
        })
        .expect("Could not create graphics pipeline");

    let mut color = ModelPushConstant {
        mat: Matrix4::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), Deg(0.0)),
    };

    // Egui integration
    let mut cinder_ui = Ui::new();
    let mut egui = EguiIntegration::new(&event_loop, &mut cinder, cinder_ui.visuals())
        .expect("Could not create event loop");

    let mut update_camera = false;
    let mut keyboard_state = KeyboardState::default();
    let init_time = init_start.elapsed().as_secs_f32();
    let start = Instant::now();
    event_loop.run(move |event, _, control_flow| {
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
                        // TODO: easier way to re-create render passes
                        cinder.clean_render_pass(&mut render_pass);
                        render_pass = cinder
                            .create_render_pass(RenderPassDescription {
                                color_attachments: [RenderPassAttachmentDesc::clear_store(
                                    cinder.surface_format(),
                                )
                                .with_layout_transition(LayoutTransition {
                                    initial_layout: Layout::Undefined,
                                    final_layout: Layout::ColorAttachment,
                                })],
                                depth_attachment: Some(
                                    RenderPassAttachmentDesc::clear_store(Format::D32_SFloat)
                                        .with_layout_transition(LayoutTransition {
                                            initial_layout: Layout::Undefined,
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
                                        update_camera = !update_camera;
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
                    let delta_time = start.elapsed().as_secs_f32() / 2.0;
                    update_model_push_constant(&mut color, delta_time);

                    // Main render pass
                    render_context.begin_render_pass(&cinder, &render_pass, present_index);
                    {
                        let surface_rect = cinder.surface_rect();

                        render_context.bind_graphics_pipeline(&cinder, &pipeline);
                        render_context.bind_viewport(&cinder, surface_rect);
                        render_context.bind_scissor(&cinder, surface_rect);

                        // TODO: Descriptor set will need to be different per mesh
                        render_context.bind_descriptor_sets(
                            &cinder,
                            &pipeline,
                            &[bind_group_set.set],
                        );
                        for draw in &mesh_draws {
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
                }
                render_context
                    .end(&cinder)
                    .expect("Could not end graphics context");

                cinder
                    .submit_graphics_work(&render_context, present_index)
                    .expect("Could not submit graphics work");
            }
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    // TODO: Maybe using the mouse_state concept makes more sense
                    if update_camera {
                        camera.rotate(delta);
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }

        if update_camera {
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
    });
}
