use anyhow::Result;
use cinder::{
    context::render_context::{
        AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment, RenderContext,
    },
    device::{Device, SurfaceData},
    view::View,
    Resolution,
};
use input::keyboard::VirtualKeyCode;
use math::rect::Rect2D;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const WINDOW_WIDTH: u32 = 2000;
pub const WINDOW_HEIGHT: u32 = 2000;

pub struct Renderer {
    device: Device,
    view: View,
    render_context: RenderContext,

    // TODO: Don't need to hold on to all of `SurfaceData`, most of it should be cached in `View`?
    surface_data: SurfaceData,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let device = Device::new(window)?;
        let render_context = RenderContext::new(&device)?;

        let surface_data = device.surface().get_data(
            device.p_device(),
            Resolution {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            false,
        )?;
        let view = View::new(&device, &surface_data)?;

        Ok(Self {
            device,
            view,
            render_context,
            surface_data,
        })
    }

    pub fn draw(&self) -> Result<bool> {
        let drawable = self.view.get_current_drawable(&self.device)?;

        self.render_context.begin(&self.device)?;
        {
            let surface_rect = Rect2D::from_width_height(
                self.surface_data.surface_resolution.width,
                self.surface_data.surface_resolution.height,
            );

            self.render_context
                .transition_undefined_to_color(&self.device, drawable);

            // TODO: Pretty bad, make better
            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(drawable)
                    .load_op(AttachmentLoadOp::Clear)
                    .store_op(AttachmentStoreOp::Store)
                    .layout(Layout::ColorAttachment)],
                None,
            );
            {
                self.render_context
                    .bind_viewport(&self.device, surface_rect, true);
                self.render_context.bind_scissor(&self.device, surface_rect);
            }
            self.render_context.end_rendering(&self.device);

            self.render_context
                .transition_color_to_present(&self.device, drawable);
        }
        self.render_context.end(&self.device)?;

        let is_suboptimal = self.view.present(&self.device, drawable)?;
        Ok(is_suboptimal)
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
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        match virtual_keycode {
                            VirtualKeyCode::Escape => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        }
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
