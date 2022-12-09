use std::{path::PathBuf, time::Instant};

use crate::{ui::Ui, WINDOW_HEIGHT, WINDOW_WIDTH};
use anyhow::Result;
use camera::{Camera, PerspectiveData};
use cinder::{
    cinder::{Cinder, DefaultUniformBufferObject, DefaultVertex},
    context::{
        render_context::{RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    resoruces::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupPool, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Format, ImageDescription, Usage},
        memory::{MemoryDescription, MemoryType},
        pipeline::{ColorBlendState, GraphicsPipeline, GraphicsPipelineDescription},
        sampler::Sampler,
        shader::ShaderDescription,
    },
    InitData, Resolution,
};
use egui_integration::EguiIntegration;
use ember::GpuStagingBuffer;
use input::keyboard::KeyboardState;
use math::size::Size2D;
use scene::{ImageBuffer, ObjScene};
use util::size_of_slice;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct App {
    pub cinder: Cinder,
    pub render_context: RenderContext,
    pub upload_context: UploadContext,
    pub scene: ObjScene,
    pub image_buffers: Vec<ImageBuffer>,
    pub index_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub camera: Camera,
    pub sampler: Sampler,
    pub graphics_pipeline: GraphicsPipeline,
    pub bind_group_pool: BindGroupPool,
    pub bind_group: BindGroup,
    pub cinder_ui: Ui,
    pub egui: EguiIntegration,
    pub keyboard_state: KeyboardState,
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
        let mut cinder = Cinder::new(window, init_data).expect("could not create cinder device");
        let render_context = cinder
            .create_render_context(RenderContextDescription {})
            .expect("Could not create graphics context");
        let upload_context = cinder
            .create_upload_context(UploadContextDescription {})
            .expect("could not create upload context");

        let vertex_shader = cinder
            .create_shader(ShaderDescription {
                bytes: include_bytes!("../../shaders/spv/default.vert.spv"),
            })
            .expect("Could not create vertex shader");
        let fragment_shader = cinder
            .create_shader(ShaderDescription {
                bytes: include_bytes!("../../shaders/spv/default.frag.spv"),
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

        let camera = camera::Camera::from_data(PerspectiveData::default());

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
        let (_images, image_bind_infos): (Vec<_>, Vec<_>) = image_buffers
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

        let graphics_pipeline = cinder
            .create_graphics_pipeline(GraphicsPipelineDescription {
                vertex_shader,
                fragment_shader,
                blending: ColorBlendState::add(),
                depth_testing_enabled: true,
                backface_culling: true,
                uses_depth: true,
            })
            .expect("Could not create graphics pipeline");
        let bind_group_pool = BindGroupPool::new(&cinder).unwrap();
        let bind_group = BindGroup::new(
            &cinder,
            &bind_group_pool,
            &graphics_pipeline.bind_group_layouts()[0],
            true,
        )
        .unwrap();
        bind_group.write(
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
        bind_group.write(&cinder, &image_bind_infos);

        // Egui integration
        let cinder_ui = Ui::new();
        let egui = EguiIntegration::new(
            &event_loop,
            &mut cinder,
            cinder_ui.visuals(),
            cinder_ui.ui_scale(),
        )
        .expect("Could not create event loop");
        let keyboard_state = KeyboardState::default();

        Ok(App {
            cinder,
            render_context,
            upload_context,
            scene,
            image_buffers,
            index_buffer,
            vertex_buffer,
            uniform_buffer,
            camera,
            sampler,
            graphics_pipeline,
            bind_group_pool,
            bind_group,
            cinder_ui,
            egui,
            keyboard_state,
        })
    }
}
