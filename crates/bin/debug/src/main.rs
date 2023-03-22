use anyhow::Result;
use cinder::{
    context::{
        render_context::{Layout, RenderAttachment, RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    device::Device,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Image, ImageDescription},
        manager::ResourceHandle,
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::{Sampler, SamplerDescription},
        shader::ShaderDesc,
        ResourceManager,
    },
    view::{View, ViewDescription},
    ResourceId,
};
use math::size::Size2D;
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
    "/gen/debug_shader_structs.rs"
));

pub struct Renderer {
    resource_manager: ResourceManager,
    device: Device,
    view: View,
    render_pipeline: ResourceHandle<GraphicsPipeline>,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer_handle: ResourceHandle<Buffer>,
    index_buffer_handle: ResourceHandle<Buffer>,
    _image_buffer_handle: ResourceHandle<Buffer>,
    _sampler: ResourceHandle<Sampler>,
    _texture_handle: ResourceHandle<Image>,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let mut resource_manager = ResourceManager::default();
        let device = Device::new(window, Default::default())?;
        let render_context = RenderContext::new(
            &device,
            RenderContextDescription {
                name: Some("render context"),
            },
        )?;
        let upload_context = UploadContext::new(
            &device,
            UploadContextDescription {
                name: Some("upload context"),
            },
        )?;

        let view = View::new(
            &device,
            ViewDescription {
                name: Some("debug view"),
            },
        )?;

        let vertex_shader = resource_manager.insert_shader(device.create_shader(
            include_bytes!("../shaders/spv/debug.vert.spv"),
            ShaderDesc {
                name: Some("vertex shader"),
            },
        )?);
        let fragment_shader = resource_manager.insert_shader(device.create_shader(
            include_bytes!("../shaders/spv/debug.frag.spv"),
            ShaderDesc {
                name: Some("fragment shader"),
            },
        )?);
        let render_pipeline = device.create_graphics_pipeline(
            &mut resource_manager,
            vertex_shader,
            fragment_shader,
            GraphicsPipelineDescription {
                name: Some("debug pipeline"),
                ..Default::default()
            },
        )?;

        let vertex_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[
                DebugVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                DebugVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                DebugVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                DebugVertex {
                    i_pos: [-0.5, 0.5],
                    i_uv: [0.0, 0.0],
                },
            ],
            BufferDescription {
                name: Some("vertex buffer"),
                usage: BufferUsage::VERTEX,
                ..Default::default()
            },
        )?);
        let index_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &[0, 1, 2, 2, 3, 0],
            BufferDescription {
                name: Some("index buffer"),
                usage: BufferUsage::INDEX,
                ..Default::default()
            },
        )?);

        let sampler = resource_manager.insert_sampler(device.create_sampler(
            &device,
            SamplerDescription {
                name: Some("sampler"),
                ..Default::default()
            },
        )?);

        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let texture_handle = resource_manager.insert_image(device.create_image(
            Size2D::new(width, height),
            ImageDescription {
                name: Some("debug image"),
                ..Default::default()
            },
        )?);
        let image_data = image.into_raw();

        let image_buffer_handle = resource_manager.insert_buffer(device.create_buffer_with_data(
            &image_data,
            BufferDescription {
                name: Some("image buffer"),
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
        )?);
        let image_buffer = resource_manager
            .get_buffer(image_buffer_handle.id())
            .unwrap();
        let texture = resource_manager.get_image(texture_handle.id()).unwrap();
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

        let pipeline = resource_manager
            .get_graphics_pipeline(render_pipeline.id())
            .unwrap();
        let s = resource_manager.get_sampler(sampler.id()).unwrap();
        device.write_bind_group(
            pipeline,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(texture.bind_info(
                    s,
                    Layout::ShaderReadOnly,
                    0,
                )),
            }],
        )?;

        Ok(Self {
            resource_manager,
            device,
            view,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            vertex_buffer_handle,
            index_buffer_handle,
            _image_buffer_handle: image_buffer_handle,
            _sampler: sampler,
            _texture_handle: texture_handle,
        })
    }

    pub fn draw(&mut self) -> Result<bool> {
        let drawable = self.view.get_current_drawable(&self.device)?;

        self.device.begin_queue_label(
            "present queue submission",
            [94.0 / 255.0, 3.0 / 255.0, 252.0 / 255.0, 1.0],
        );
        self.render_context.begin(&self.device)?;
        {
            let surface_rect = self.device.surface_rect();

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            self.render_context.begin_debug_label(
                &self.device,
                "begin rendering",
                [0.44, 1.0, 0.5, 1.0],
            );
            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable, Default::default())],
                None,
            );
            {
                let pipeline = self
                    .resource_manager
                    .get_graphics_pipeline(self.render_pipeline.id())
                    .unwrap();
                self.render_context
                    .bind_graphics_pipeline(&self.device, pipeline);
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                let index_buffer = self
                    .resource_manager
                    .get_buffer(self.index_buffer_handle.id())
                    .unwrap();
                self.render_context
                    .bind_index_buffer(&self.device, index_buffer);
                let vertex_buffer = self
                    .resource_manager
                    .get_buffer(self.vertex_buffer_handle.id())
                    .unwrap();
                self.render_context
                    .bind_vertex_buffer(&self.device, vertex_buffer);
                self.render_context
                    .bind_descriptor_sets(&self.device, pipeline);

                self.render_context.insert_label(
                    &self.device,
                    "draw",
                    [252.0 / 255.0, 186.0 / 255.0, 3.0 / 255.0, 1.0],
                );
                self.render_context.draw_offset(&self.device, 6, 0, 0);
            }
            self.render_context.end_rendering(&self.device);
            self.render_context.end_debug_label(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;
        self.device.end_queue_label();

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
