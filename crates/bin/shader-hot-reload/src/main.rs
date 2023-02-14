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
    },
    view::View,
    ResourceHandle,
};
use math::size::Size2D;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
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
}

pub struct ShaderHotReloaderRunner {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<notify::event::Event, notify::Error>>,
    event_handlers: HashMap<
        PathBuf,
        (
            ResourceHandle<()>,
            Box<dyn FnMut(notify::event::Event) + Send>,
        ),
    >,
}

impl ShaderHotReloaderRunner {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher = notify::Watcher::new(sender, Default::default())?;
        Ok(Self {
            watcher,
            receiver,
            event_handlers: Default::default(),
        })
    }

    pub fn set_graphics(
        &mut self,
        vertex: impl AsRef<Path>,
        fragment: impl AsRef<Path>,
        program_handle: ResourceHandle<GraphicsPipeline>,
    ) -> Result<()> {
        let vertex = Path::new(env!("CARGO_MANIFEST_DIR")).join(&vertex);
        let fragment = Path::new(env!("CARGO_MANIFEST_DIR")).join(&fragment);
        self.watcher.watch(&vertex, RecursiveMode::NonRecursive)?;
        self.watcher.watch(&fragment, RecursiveMode::NonRecursive)?;
        self.event_handlers.insert(
            vertex.canonicalize()?,
            (
                program_handle.as_unit(),
                Box::new(|event| {
                    println!("Frag: {event:?}");
                }),
            ),
        );
        self.event_handlers.insert(
            fragment.canonicalize()?,
            (
                program_handle.as_unit(),
                Box::new(|event| {
                    println!("Vert: {event:?}");
                }),
            ),
        );
        Ok(())
    }

    pub fn run(self) -> ShaderHotReloader {
        let Self {
            watcher,
            receiver,
            mut event_handlers,
        } = self;

        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(event) => {
                    match event {
                        Ok(event) => {
                            for path in &event.paths {
                                match path.canonicalize() {
                                    Ok(path) => {
                                        if let Some((handle, handler)) =
                                            event_handlers.get_mut(&path)
                                        {
                                            handler(event.clone());
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
        ShaderHotReloader { watcher }
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

        let mut vertex_shader = device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.vert.spv"),
            Default::default(),
        )?;
        let mut fragment_shader = device.create_shader(
            include_bytes!("../shaders/spv/hot_reload.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &vertex_shader,
            &fragment_shader,
            Default::default(),
        )?;
        shader_hot_reloader.set_graphics(
            "shaders/hot_reload.frag",
            "shaders/hot_reload.vert",
            render_pipeline,
        )?;
        vertex_shader.destroy(&device);
        fragment_shader.destroy(&device);

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
