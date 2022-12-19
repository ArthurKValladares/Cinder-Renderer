mod runtime;

use crate::{renderer::Renderer, MeshDraw, WINDOW_HEIGHT, WINDOW_WIDTH};
use anyhow::Result;
use camera::{MOVEMENT_DELTA, ROTATION_DELTA};
use cinder::{
    cinder::{DefaultUniformBufferObject, DefaultVertex},
    context::{
        render_context::{
            AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment, RenderContext,
            RenderContextDescription,
        },
        upload_context::{UploadContext, UploadContextDescription},
    },
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupPool, BindGroupWriteData},
        buffer::{vk, Buffer, BufferDescription, BufferUsage},
        image::{Format, ImageDescription, ImageViewDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{
            compute::{ComputePipeline, ComputePipelineDescription},
            graphics::{ColorBlendState, GraphicsPipeline, GraphicsPipelineDescription},
        },
        sampler::Sampler,
        shader::{ShaderDescription, ShaderStage},
    },
    InitData, Resolution,
};
use egui_integration::egui;
use input::keyboard::{ElementState, VirtualKeyCode};
use math::size::Size2D;
use scene::{ImageBuffer, ObjScene};
use std::{path::PathBuf, time::Instant};
use util::size_of_slice;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use self::runtime::RuntimeState;

pub struct App {
    pub renderer: Renderer,
    pub render_context: RenderContext,
    pub upload_context: UploadContext,
    pub scene: ObjScene,
    pub scene_load_time: f32,
    pub image_buffers: Vec<ImageBuffer>,
    pub index_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub sampler: Sampler,
    pub graphics_pipeline: GraphicsPipeline,
    pub compute_pipeline: ComputePipeline,
    pub bind_group_pool: BindGroupPool,
    pub bind_group: BindGroup,
    pub runtime_state: RuntimeState,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>, window: &Window) -> Result<Self> {
        let init_data = InitData {
            backbuffer_resolution: Resolution {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            vsync: true,
        };
        let mut renderer = Renderer::new(window, init_data)?;
        let render_context = renderer.create_render_context(RenderContextDescription {})?;
        let upload_context = renderer.create_upload_context(UploadContextDescription {})?;

        let vertex_shader = renderer.device().create_shader(ShaderDescription {
            bytes: include_bytes!("../../shaders/spv/default.vert.spv"),
        })?;
        let fragment_shader = renderer.device().create_shader(ShaderDescription {
            bytes: include_bytes!("../../shaders/spv/default.frag.spv"),
        })?;
        let compute_shader = renderer.device().create_shader(ShaderDescription {
            bytes: include_bytes!("../../shaders/spv/depth_reduce.comp.spv"),
        })?;
        // Load model
        let scene_load_start = Instant::now();
        let (scene, image_buffers) = scene::ObjScene::load_or_achive(
            PathBuf::from("assets").join("models").join("sponza"),
            "sponza.obj",
        )
        .unwrap_or_else(|err| panic!("Could not load mesh: {err}"));
        let scene_load_time = scene_load_start.elapsed().as_secs_f32();

        let (num_vertices, num_indices) =
            scene.meshes.iter().fold((0, 0), |(n_vert, n_index), mesh| {
                (n_vert + mesh.vertices.len(), n_index + mesh.indices.len())
            });

        let index_buffer = renderer
            .device()
            .create_buffer(BufferDescription {
                size: (num_indices * std::mem::size_of::<u32>()) as u64,
                usage: BufferUsage::empty().index().transfer_dst(),
                memory_desc: MemoryDescription {
                    ty: MemoryType::GpuOnly,
                },
            })
            .expect("Could not create index buffer");
        let vertex_buffer = renderer
            .device()
            .create_buffer(BufferDescription {
                size: (num_vertices * std::mem::size_of::<DefaultVertex>()) as u64,
                usage: BufferUsage::empty().storage().transfer_dst(),
                memory_desc: MemoryDescription {
                    ty: MemoryType::GpuOnly,
                },
            })
            .expect("Could not create vertex buffer");

        let uniform_buffer = renderer
            .device()
            .create_buffer(BufferDescription {
                size: std::mem::size_of::<DefaultUniformBufferObject>() as u64,
                usage: BufferUsage::empty().uniform(),
                memory_desc: MemoryDescription {
                    ty: MemoryType::CpuVisible,
                },
            })
            .expect("Could not create uniform buffer");

        let sampler = renderer
            .device()
            .create_sampler()
            .expect("Could not create sampler");

        upload_context
            .begin(renderer.device(), renderer.setup_fence())
            .expect("could not begin upload context");
        // Create and upload image
        let (_images, image_bind_infos): (Vec<_>, Vec<_>) = image_buffers
            .iter()
            .enumerate()
            .map(|(idx, image)| {
                let image_buffer = renderer
                    .device()
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

                let mut texture = renderer
                    .device()
                    .create_image(ImageDescription {
                        format: Format::R8_G8_B8_A8_Unorm,
                        usage: Usage::Texture,
                        size: Size2D::new(image.width, image.height),
                    })
                    .expect("could not create texture");
                let image_view_desc = ImageViewDescription {
                    format: Format::R8_G8_B8_A8_Unorm,
                    usage: Usage::Texture,
                };
                texture
                    .add_view(renderer.device(), image_view_desc)
                    .unwrap();
                upload_context.image_barrier_start(renderer.device(), &texture);
                upload_context.copy_buffer_to_image(renderer.device(), &image_buffer, &texture);
                upload_context.image_barrier_end(renderer.device(), &texture);

                let info = texture.bind_info(&sampler, image_view_desc, idx as u32);

                (
                    texture,
                    BindGroupBindInfo {
                        dst_binding: 2,
                        data: BindGroupWriteData::Image(info),
                    },
                )
            })
            .unzip();
        upload_context.transition_depth_image(renderer.device(), renderer.depth_image());
        upload_context
            .end(
                renderer.device(),
                renderer.setup_fence(),
                renderer.present_queue(),
                &[],
                &[],
                &[],
            )
            .expect("could not end upload context");

        let vertex_buffer_info = vertex_buffer.bind_info();
        let uniform_buffer_info = uniform_buffer.bind_info();

        let graphics_pipeline = renderer
            .device()
            .create_graphics_pipeline(
                GraphicsPipelineDescription {
                    vertex_shader,
                    fragment_shader,
                    blending: ColorBlendState::add(),
                    backface_culling: true,
                    surface_format: renderer.surface_format(),
                    depth_format: Some(Format::D32_SFloat),
                },
                Some(renderer.pipeline_cache()),
            )
            .expect("Could not create graphics pipeline");
        let compute_pipeline = renderer
            .device()
            .create_compute_pipeline(
                ComputePipelineDescription {
                    shader: compute_shader,
                },
                Some(renderer.pipeline_cache()),
            )
            .expect("Could not create graphics pipeline");

        let bind_group_pool = BindGroupPool::new(renderer.device()).unwrap();
        let bind_group = BindGroup::new(
            renderer.device(),
            &bind_group_pool,
            &graphics_pipeline.bind_group_layouts()[0],
            true,
        )
        .unwrap();
        bind_group.write(
            renderer.device(),
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
        bind_group.write(renderer.device(), &image_bind_infos);

        let runtime_state = RuntimeState::new(event_loop, &mut renderer);

        Ok(App {
            renderer,
            render_context,
            upload_context,
            scene,
            scene_load_time,
            image_buffers,
            index_buffer,
            vertex_buffer,
            uniform_buffer,
            sampler,
            graphics_pipeline,
            compute_pipeline,
            bind_group_pool,
            bind_group,
            runtime_state,
        })
    }

    pub fn run(
        mut self,
        window: Window,
        event_loop: EventLoop<()>,
        mesh_draws: Vec<MeshDraw>, // TODO: This is bad
    ) -> ! {
        let mut lock_movement = true;
        let start = Instant::now();
        let mut frame_cpu_average = f32::MAX;
        let mut frame_gpu_average = f32::MAX;
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            let frame_start = Instant::now();
            //
            // Update
            //
            self.runtime_state.mouse_state.update(&event);
            if !lock_movement {
                self.runtime_state.update_position();
                self.runtime_state.rotate_camera();
            }

            // TODO: Dont need to update every frame, only when camera changes
            self.uniform_buffer
                .mem_copy(
                    0,
                    std::slice::from_ref(
                        &self
                            .runtime_state
                            .get_camera_matrices(self.renderer.surface_size()),
                    ),
                )
                .expect("Could not write to uniform buffer");

            //
            // Render
            //
            match event {
                Event::WindowEvent {
                    event: window_event,
                    ..
                } => {
                    self.runtime_state.poll_event(&window_event);
                    match window_event {
                        WindowEvent::Resized(size) => {
                            self.renderer
                                .resize(Size2D::new(size.width, size.height))
                                .expect("Could not resize device");
                            self.runtime_state
                                .resize(&self.renderer)
                                .expect("could not resize RuntimeState");
                            // TODO: This could be better
                            self.upload_context
                                .begin(self.renderer.device(), self.renderer.setup_fence())
                                .expect("could not begin upload context");
                            {
                                self.upload_context.transition_depth_image(
                                    self.renderer.device(),
                                    self.renderer.depth_image(),
                                );
                            }
                            self.upload_context
                                .end(
                                    self.renderer.device(),
                                    self.renderer.setup_fence(),
                                    self.renderer.present_queue(),
                                    &[],
                                    &[],
                                    &[],
                                )
                                .expect("could not end upload context");
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            self.runtime_state.update_keyboard_state(input);
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
                    // TODO: Handle is_suboptimal
                    let (present_index, _is_suboptimal) = self
                        .renderer
                        .acquire_next_image()
                        .expect("Could not acquire swapchain image");

                    self.render_context
                        .begin(self.renderer.device(), self.renderer.draw_fence())
                        .expect("Could not begin graphics context");
                    {
                        self.render_context.reset_query_pool(
                            self.renderer.device(),
                            &self.renderer.profiling.timestamp_query_pool,
                        );
                        self.render_context.write_timestamp(
                            self.renderer.device(),
                            &self.renderer.profiling.timestamp_query_pool,
                            0,
                        );

                        let delta_time = start.elapsed().as_secs_f32();
                        let color = [delta_time.sin() / 2.0, 0.0, 0.0, 0.0];

                        let surface_rect = self.renderer.surface_rect();

                        self.render_context.transition_undefined_to_color(
                            self.renderer.device(),
                            self.renderer.swapchain(),
                            present_index,
                        );

                        // TODO: Pretty bad, make better
                        self.render_context.begin_rendering(
                            self.renderer.device(),
                            surface_rect,
                            &[
                                RenderAttachment::color(self.renderer.swapchain(), present_index)
                                    .load_op(AttachmentLoadOp::Clear)
                                    .store_op(AttachmentStoreOp::Store)
                                    .layout(Layout::ColorAttachment),
                            ],
                            Some(
                                RenderAttachment::depth(
                                    self.renderer.depth_image(),
                                    ImageViewDescription {
                                        format: Format::D32_SFloat,
                                        usage: Usage::Depth,
                                    },
                                )
                                .load_op(AttachmentLoadOp::Clear)
                                .store_op(AttachmentStoreOp::DontCare)
                                .layout(Layout::DepthAttachment),
                            ),
                        );
                        {
                            self.render_context.bind_graphics_pipeline(
                                self.renderer.device(),
                                &self.graphics_pipeline,
                            );
                            self.render_context.bind_viewport(
                                self.renderer.device(),
                                surface_rect,
                                true,
                            );
                            self.render_context
                                .bind_scissor(self.renderer.device(), surface_rect);
                            self.render_context
                                .bind_index_buffer(self.renderer.device(), &self.index_buffer);
                            self.render_context.bind_descriptor_sets(
                                self.renderer.device(),
                                &self.graphics_pipeline,
                                &[self.bind_group.0],
                            );

                            for draw in &mesh_draws {
                                self.render_context
                                    .push_constant(
                                        self.renderer.device(),
                                        &self.graphics_pipeline,
                                        ShaderStage::Vertex,
                                        0,
                                        util::as_u8_slice(&color),
                                    )
                                    .unwrap();

                                self.render_context
                                    .push_constant(
                                        self.renderer.device(),
                                        &self.graphics_pipeline,
                                        ShaderStage::Fragment,
                                        0,
                                        util::as_u8_slice(&draw.image_index),
                                    )
                                    .unwrap();

                                self.render_context.draw_offset(
                                    self.renderer.device(),
                                    draw.num_indices as u32,
                                    draw.index_buffer_offset,
                                    draw.vertex_buffer_offset,
                                );
                            }
                        }
                        self.render_context.end_rendering(self.renderer.device());

                        // Ui/egui render pass
                        self.runtime_state
                            .egui
                            .run(
                                self.renderer.device(),
                                self.renderer.swapchain(),
                                &self.upload_context,
                                self.renderer.setup_fence(),
                                &self.render_context,
                                self.renderer.surface_rect(),
                                present_index,
                                &window,
                                |egui_context| {
                                    egui::TopBottomPanel::top("Cinder").show(egui_context, |ui| {
                                        self.runtime_state.ui.show_tabs(ui);
                                    });

                                    // TODO: Move this logic to RuntimeState
                                    self.runtime_state.ui.show_selected_tab(
                                        egui_context,
                                        self.renderer.surface_size(),
                                        |ui| {
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
                                                    "Average CPU: {frame_cpu_average:.5} ms",
                                                ));
                                                ui.label(format!(
                                                    "Average GPU: {frame_gpu_average:.5} ms",
                                                ));
                                            });
                                            ui.collapsing("init", |ui| {
                                                // TODO: Better time profiling tool
                                                ui.label(format!(
                                                    "scene load: {} s",
                                                    self.scene_load_time
                                                ));
                                            });
                                            ui.collapsing("camera", |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label("movement speed: ");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut self
                                                                .runtime_state
                                                                .camera
                                                                .movement_speed,
                                                        )
                                                        .speed(MOVEMENT_DELTA),
                                                    );
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("rotation speed: ");
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut self
                                                                .runtime_state
                                                                .camera
                                                                .rotation_speed,
                                                        )
                                                        .speed(ROTATION_DELTA),
                                                    );
                                                });
                                            });
                                        },
                                    )
                                },
                            )
                            .expect("Could not run egui");

                        self.render_context.write_timestamp(
                            self.renderer.device(),
                            &self.renderer.profiling.timestamp_query_pool,
                            1,
                        );

                        self.render_context.transition_color_to_present(
                            self.renderer.device(),
                            self.renderer.swapchain(),
                            present_index,
                        );
                    }
                    self.render_context
                        .end(
                            self.renderer.device(),
                            self.renderer.draw_fence(),
                            self.renderer.present_queue(),
                            &[vk::PipelineStageFlags::BOTTOM_OF_PIPE], // TODO: Abstract later
                            &[self.renderer.present_semaphore()],
                            &[self.renderer.render_semaphore()],
                        )
                        .expect("Could not end graphics context");
                    self.renderer
                        .present(present_index)
                        .expect("Could not submit graphics work");
                }
                _ => {}
            }

            let frame_dt = frame_start.elapsed().as_millis() as f32;
            if frame_cpu_average == f32::MAX {
                frame_cpu_average = frame_dt;
            } else {
                frame_cpu_average = frame_cpu_average * 0.95 + frame_dt * 0.05;
            }

            let timestamp_results = self
                .renderer
                .device()
                .get_query_pool_results_u64(&self.renderer.profiling.timestamp_query_pool, 0, 2)
                .unwrap_or_else(|_| vec![]);
            if !timestamp_results.is_empty() {
                let gpu_begin = timestamp_results[0] as f32
                    * self.renderer.device().properties().limits.timestamp_period
                    * 1e-6;
                let gpu_end = timestamp_results[1] as f32
                    * self.renderer.device().properties().limits.timestamp_period
                    * 1e-6;
                let gpu_dt = gpu_end - gpu_begin;
                if frame_gpu_average == f32::MAX {
                    frame_gpu_average = gpu_dt;
                } else {
                    frame_gpu_average = frame_gpu_average * 0.95 + gpu_dt * 0.05;
                }
            }

            window.request_redraw();
        })
    }
}
