use std::path::Path;

use anyhow::Result;
use cinder::{
    context::{
        render_context::{Layout, RenderAttachment, RenderContext},
        upload_context::UploadContext,
    },
    device::{Device, ResourceManager},
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
use shader_hot_reloader::{ShaderHotReloader, ShaderHotReloaderRunner};
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

pub struct Renderer {
    shader_hot_reloader: ShaderHotReloader,
    resource_manager: ResourceManager,
    device: Device,
    view: View,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer_handle: ResourceHandle<Buffer>,
    index_buffer_handle: ResourceHandle<Buffer>,
    image_buffer_handle: ResourceHandle<Buffer>,
    sampler: Sampler,
    texture_handle: ResourceHandle<Image>,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut shader_hot_reloader = ShaderHotReloaderRunner::new()?;

        let mut resource_manager = ResourceManager::default();
        let device = Device::new(window, Default::default())?;
        let render_context = RenderContext::new(&device, Default::default())?;
        let upload_context = UploadContext::new(&device, Default::default())?;

        let view = View::new(&device, Default::default())?;

        let vertex_shader = device.create_shader(
            &mut resource_manager,
            include_bytes!("../shaders/spv/hot_reload.vert.spv"),
            Default::default(),
        )?;
        let fragment_shader = device.create_shader(
            &mut resource_manager,
            include_bytes!("../shaders/spv/hot_reload.frag.spv"),
            Default::default(),
        )?;
        let render_pipeline = device.create_graphics_pipeline(
            &mut resource_manager,
            vertex_shader,
            fragment_shader,
            Default::default(),
        )?;
        shader_hot_reloader.set_graphics(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.vert")
                .canonicalize()?,
            vertex_shader,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("shaders")
                .join("hot_reload.frag")
                .canonicalize()?,
            fragment_shader,
            render_pipeline,
        )?;

        let vertex_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
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
        let index_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
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
        let texture_handle = device.create_image(
            &mut resource_manager,
            Size2D::new(width, height),
            Default::default(),
        )?;
        let image_data = image.into_raw();

        let image_buffer_handle = device.create_buffer_with_data(
            &mut resource_manager,
            &image_data,
            BufferDescription {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
        )?;
        let image_buffer = resource_manager.get_buffer(image_buffer_handle).unwrap();
        let texture = resource_manager.get_image(texture_handle).unwrap();
        upload_context.begin(&device, device.setup_fence())?;
        {
            upload_context.image_barrier_start(&device, texture);
            upload_context.copy_buffer_to_image(&device, image_buffer, texture);
            upload_context.image_barrier_end(&device, texture);
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
            &resource_manager,
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
            resource_manager,
            device,
            view,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            vertex_buffer_handle,
            index_buffer_handle,
            image_buffer_handle,
            sampler,
            texture_handle,
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
                self.render_context.bind_graphics_pipeline(
                    &self.resource_manager,
                    &self.device,
                    self.render_pipeline,
                )?;
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                let index_buffer = self
                    .resource_manager
                    .get_buffer(self.index_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, index_buffer);
                let vertex_buffer = self
                    .resource_manager
                    .get_buffer(self.vertex_buffer_handle)
                    .unwrap();
                self.render_context
                    .bind_vertex_buffer(&self.device, vertex_buffer);
                self.render_context
                    .bind_descriptor_sets(&self.resource_manager, &self.device)?;

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

    pub fn update(&mut self) -> Result<()> {
        for update_data in self.shader_hot_reloader.drain()? {
            if let Some(pipeline_handle) = self
                .shader_hot_reloader
                .get_pipeline(update_data.shader_handle)
            {
                self.device.recreate_shader(
                    &mut self.resource_manager,
                    &update_data.bytes,
                    update_data.shader_handle,
                );
                self.device
                    .recreate_graphics_pipeline(&mut self.resource_manager, *pipeline_handle)?;
            }
        }
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.device.wait_idle().ok();

        self.sampler.destroy(self.device.raw());

        self.view.destroy(&self.device);
        self.resource_manager.clean(&self.device);
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

        renderer.update().expect("could not update renderer");

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
