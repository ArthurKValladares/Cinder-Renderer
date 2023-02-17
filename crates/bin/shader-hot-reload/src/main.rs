use anyhow::Result;
use cinder::{
    context::{
        render_context::{Layout, RenderAttachment, RenderContext},
        upload_context::UploadContext,
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::Image,
        pipeline::graphics::GraphicsPipeline,
        sampler::Sampler,
        shader::Shader,
    },
    view::View,
    ResourceHandle,
};
use math::size::Size2D;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rust_shader_tools::{EnvVersion, OptimizationLevel, ShaderCompiler, ShaderStage};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{mpsc::Receiver, Arc, Mutex, MutexGuard},
};
use winit::{
    dpi::PhysicalSize,
    event::VirtualKeyCode,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const WINDOW_WIDTH: u32 = 2000;
pub const WINDOW_HEIGHT: u32 = 2000;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/gen/hot_reload_shader_structs.rs"
));

pub struct ShaderHotReloader {
    watcher: RecommendedWatcher,
    program_map: HashMap<ResourceHandle<Shader>, ResourceHandle<GraphicsPipeline>>,
    // TODO: If I make the Device theread-safe, I don't need this
    // TODO: Right now I onlty have the binary data for one shader, need to keep the other one around to re-create pipeline
    to_be_updated: Arc<Mutex<Vec<(ResourceHandle<Shader>, Vec<u8>)>>>,
}

pub struct ShaderHotReloaderRunner {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<notify::event::Event, notify::Error>>,
    shader_map: HashMap<PathBuf, (ResourceHandle<Shader>, ShaderStage)>,
    program_map: HashMap<ResourceHandle<Shader>, ResourceHandle<GraphicsPipeline>>,
}

impl ShaderHotReloaderRunner {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher = notify::Watcher::new(sender, Default::default())?;
        Ok(Self {
            watcher,
            receiver,
            shader_map: Default::default(),
            program_map: Default::default(),
        })
    }

    pub fn set_graphics(
        &mut self,
        vertex: impl AsRef<Path>,
        vertex_handle: ResourceHandle<Shader>,
        fragment: impl AsRef<Path>,
        fragment_handle: ResourceHandle<Shader>,
        program_handle: ResourceHandle<GraphicsPipeline>,
    ) -> Result<()> {
        let vertex = Path::new(env!("CARGO_MANIFEST_DIR")).join(&vertex);
        self.watcher.watch(&vertex, RecursiveMode::NonRecursive)?;
        self.shader_map
            .insert(vertex.canonicalize()?, (vertex_handle, ShaderStage::Vertex));
        self.program_map.insert(vertex_handle, program_handle);

        let fragment = Path::new(env!("CARGO_MANIFEST_DIR")).join(&fragment);
        self.watcher.watch(&fragment, RecursiveMode::NonRecursive)?;
        self.shader_map.insert(
            fragment.canonicalize()?,
            (fragment_handle, ShaderStage::Fragment),
        );
        self.program_map.insert(fragment_handle, program_handle);

        Ok(())
    }

    pub fn run(self) -> ShaderHotReloader {
        let Self {
            watcher,
            receiver,
            mut shader_map,
            program_map,
        } = self;

        let shader_compiler =
            ShaderCompiler::new(EnvVersion::Vulkan1_2, OptimizationLevel::Zero, None)
                .expect("Could not create shader compiler");
        let to_be_updated = Arc::<Mutex<_>>::default();
        let to_be_updated_arc = Arc::clone(&to_be_updated);
        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(event) => {
                    match event {
                        Ok(event) => {
                            for path in &event.paths {
                                match path.canonicalize() {
                                    Ok(path) => {
                                        if let Some((handle, stage)) = shader_map.get_mut(&path) {
                                            println!("{path:?} {stage:?}");
                                            let artifact = shader_compiler
                                                .compile_shader(&path, *stage)
                                                .expect("failed to compiler shader");
                                            let mut lock: MutexGuard<
                                                Vec<(ResourceHandle<Shader>, Vec<u8>)>,
                                            > = to_be_updated_arc
                                                .lock()
                                                .expect("mutex lock poisoned");
                                            lock.push((*handle, artifact.as_binary_u8().to_vec()));
                                        }
                                    }
                                    Err(err) => {
                                        println!("Shader hot-reload error: {err:?}");
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            println!("Shader hot-reload error: {err:?}");
                        }
                    };
                }
                Err(_) => {
                    println!("Shader Hot-Reloader Stopped");
                    break;
                }
            }
        });

        ShaderHotReloader {
            watcher,
            program_map,
            to_be_updated,
        }
    }
}

pub struct Renderer {
    shader_hot_reloader: ShaderHotReloader,
    device: Device,
    view: View,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    image_buffer: Buffer,
    sampler: Sampler,
    texture: Image,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut shader_hot_reloader = ShaderHotReloaderRunner::new()?;

        let mut device = Device::new(window, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let upload_context = UploadContext::new(&device, Default::default())?;

        let view = View::new(&device, Default::default())?;

        let vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline =
            device.create_graphics_pipeline(vertex_shader, fragment_shader, Default::default())?;
        shader_hot_reloader.set_graphics(
            "shaders/hot_reload.vert",
            vertex_shader,
            "shaders/hot_reload.frag",
            fragment_shader,
            render_pipeline,
        )?;

        let vertex_buffer = device.create_buffer_with_data(
            &[
                HotReloadVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                HotReloadVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                HotReloadVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?;
        let index_buffer = device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?;

        let sampler = device.create_sampler(&device, Default::default())?;

        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let texture = device.create_image(Size2D::new(width, height), Default::default())?;
        let image_data = image.into_raw();

        let image_buffer = device.create_buffer_with_data(
            &image_data,
            BufferDescription {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
        )?;
        upload_context.begin(&device, device.setup_fence())?;
        {
            upload_context.image_barrier_start(&device, &texture);
            upload_context.copy_buffer_to_image(&device, &image_buffer, &texture);
            upload_context.image_barrier_end(&device, &texture);
        }
        upload_context.end(
            &device,
            device.setup_fence(),
            device.present_queue(),
            &[],
            &[],
            &[],
        )?;

        device.write_bind_group(
            render_pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(texture.bind_info(
                    &sampler,
                    Layout::ShaderReadOnly,
                    0,
                )),
            }],
        )?;

        Ok(Self {
            shader_hot_reloader: shader_hot_reloader.run(),
            device,
            view,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            image_buffer,
            sampler,
            texture,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let drawable = self.view.get_current_drawable(&self.device)?;

        self.render_context.begin(&self.device)?;
        {
            let surface_rect = self.device.surface_rect();

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable, Default::default())],
                None,
            );
            {
                self.render_context
                    .bind_graphics_pipeline(&self.device, self.render_pipeline)?;
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                self.render_context
                    .bind_index_buffer(&self.device, &self.index_buffer);
                self.render_context
                    .bind_vertex_buffer(&self.device, &self.vertex_buffer);
                self.render_context.bind_descriptor_sets(&self.device)?;

                self.render_context.draw_offset(&self.device, 6, 0, 0);
            }
            self.render_context.end_rendering(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;

        self.view.present(&self.device, drawable)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.device.resize(width, height)?;
        self.view.resize(&self.device)?;
        Ok(())
    }

    pub fn update(&mut self) {
        let mut lock: MutexGuard<Vec<(ResourceHandle<Shader>, Vec<u8>)>> = self
            .shader_hot_reloader
            .to_be_updated
            .lock()
            .expect("mutex lock poisoned");
        for (shader_handle, bytes) in lock.drain(..) {
            self.device.recreate_shader(&bytes, shader_handle);
            if let Some(pipeline_handle) = self.shader_hot_reloader.program_map.get(&shader_handle)
            {
                self.device.recreate_graphics_pipeline(*pipeline_handle);
            }
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();

        self.sampler.destroy(self.device.raw());
        self.texture.destroy(self.device.raw());

        self.vertex_buffer.destroy(self.device.raw());
        self.index_buffer.destroy(self.device.raw());
        self.image_buffer.destroy(self.device.raw());

        self.view.destroy(&self.device);
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cinder Window")
        .with_inner_size(PhysicalSize {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let mut renderer = Renderer::new(&window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::Resized(size) => {
                    renderer.resize(size.width, size.height).unwrap();
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            Event::RedrawRequested(_) => {
                renderer.draw().unwrap();
            }
            _ => {}
        }

        window.request_redraw();
    });
}
