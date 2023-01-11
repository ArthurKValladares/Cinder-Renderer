use anyhow::Result;
use cinder::{
    context::{
        render_context::{RenderAttachment, RenderContext},
        upload_context::UploadContext,
    },
    device::{Device, SurfaceData},
    resources::{
        bind_group::{BindGroup, BindGroupBindInfo, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::Image,
        pipeline::graphics::GraphicsPipeline,
        sampler::Sampler,
    },
    view::View,
    Resolution,
};
use math::{rect::Rect2D, size::Size2D};
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
    "/gen/texture_shader_structs.rs"
));

pub struct Renderer {
    device: Device,
    view: View,
    render_pipeline: GraphicsPipeline,
    render_bind_group: BindGroup,
    render_context: RenderContext,
    _upload_context: UploadContext,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    _sampler: Sampler,
    _texture: Image,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let device = Device::new(window)?;
        let render_context = RenderContext::new(&device)?;
        let upload_context = UploadContext::new(&device)?;

        let view = View::new(&device)?;

        let render_pipeline = device.create_graphics_pipeline(
            device.create_shader(include_bytes!("../shaders/spv/texture.vert.spv"))?,
            device.create_shader(include_bytes!("../shaders/spv/texture.frag.spv"))?,
            Default::default(),
        )?;
        let render_bind_group = BindGroup::new(
            &device,
            &render_pipeline.common.bind_group_layouts()[0],
            false, // TODO: This should not be a user-side param
        )?;

        let vertex_buffer = device.create_buffer_with_data(
            &[
                TextureVertex {
                    i_pos: [-0.5, -0.5],
                    i_uv: [0.0, 1.0],
                },
                TextureVertex {
                    i_pos: [0.5, -0.5],
                    i_uv: [1.0, 1.0],
                },
                TextureVertex {
                    i_pos: [0.5, 0.5],
                    i_uv: [1.0, 0.0],
                },
                TextureVertex {
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

        let sampler = device.create_sampler()?;

        let image = image::load_from_memory(include_bytes!("../assets/rust.png"))
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let texture = device.create_image(Size2D::new(width, height), Default::default())?;
        let image_data = image.into_raw();

        // TODO: Clean up image buffer
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

        render_bind_group.write(
            &device,
            &[BindGroupBindInfo {
                dst_binding: 0,
                data: BindGroupWriteData::SampledImage(texture.bind_info(&sampler, 0)),
            }],
        );

        Ok(Self {
            device,
            view,
            render_context,
            _upload_context: upload_context,
            render_pipeline,
            render_bind_group,
            vertex_buffer,
            index_buffer,
            _sampler: sampler,
            _texture: texture,
        })
    }

    pub fn draw(&self) -> Result<bool> {
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
                    .bind_graphics_pipeline(&self.device, &self.render_pipeline);
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
                self.render_context
                    .bind_index_buffer(&self.device, &self.index_buffer);
                self.render_context
                    .bind_vertex_buffer(&self.device, &self.vertex_buffer);
                // TODO: better abstraction
                self.render_context.bind_descriptor_sets(
                    &self.device,
                    &self.render_pipeline.common,
                    &[self.render_bind_group.0],
                    false,
                );

                self.render_context.draw_offset(&self.device, 6, 0, 0);
            }
            self.render_context.end_rendering(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;

        self.view.present(&self.device, drawable)
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

    let renderer = Renderer::new(&window).unwrap();

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
