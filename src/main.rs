use anyhow::Result;
use cinder::{
    context::render_context::{
        AttachmentLoadOp, AttachmentStoreOp, Layout, RenderAttachment, RenderContext,
    },
    device::{Device, SurfaceData},
    resources::buffer::vk,
    swapchain::Swapchain,
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
    swapchain: Swapchain,
    _command_pool: vk::CommandPool,
    render_context: RenderContext,

    // TODO: Don't need to hold on to all of `SurfaceData`, most of it should be cached in `View`?
    surface_data: SurfaceData,

    // TODO: Probably will have better syncronization in the future
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
    draw_commands_reuse_fence: vk::Fence,
}

impl Renderer {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let device = Device::new(window)?;

        // TODO: Swapchain will be a part of `View`
        let surface_data = device.surface().get_data(
            device.p_device(),
            Resolution {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            false,
        )?;
        let swapchain = Swapchain::new(&device, &surface_data)?;

        // TODO: Should be a part of `Device`
        let command_pool = unsafe {
            device.raw().create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(device.queue_family_index()),
                None,
            )
        }?;

        // TODO: This should be much easier
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let render_context = RenderContext::from_command_buffer(
            unsafe {
                device
                    .raw()
                    .allocate_command_buffers(&command_buffer_allocate_info)?
            }[0],
        );

        // TODO: Abstract raw sync away from user
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let present_complete_semaphore =
            unsafe { device.raw().create_semaphore(&semaphore_create_info, None) }?;
        let rendering_complete_semaphore =
            unsafe { device.raw().create_semaphore(&semaphore_create_info, None) }?;

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let draw_commands_reuse_fence =
            unsafe { device.raw().create_fence(&fence_create_info, None) }?;

        Ok(Self {
            device,
            swapchain,
            _command_pool: command_pool,
            render_context,
            surface_data,
            present_complete_semaphore,
            rendering_complete_semaphore,
            draw_commands_reuse_fence,
        })
    }

    pub fn draw(&self) -> Result<bool> {
        // TODO: This will be abstracted in `View`, with a `get_current_drawable` kinda thing
        let (present_index, _is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        }?;

        self.render_context
            .begin(&self.device, self.draw_commands_reuse_fence)?;
        {
            let surface_rect = Rect2D::from_width_height(
                self.surface_data.surface_resolution.width,
                self.surface_data.surface_resolution.height,
            );

            self.render_context.transition_undefined_to_color(
                &self.device,
                &self.swapchain,
                present_index,
            );

            // TODO: Pretty bad, make better
            self.render_context.begin_rendering(
                &self.device,
                surface_rect,
                &[RenderAttachment::color(&self.swapchain, present_index)
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

            self.render_context.transition_color_to_present(
                &self.device,
                &self.swapchain,
                present_index,
            );
        }
        self.render_context.end(
            &self.device,
            self.draw_commands_reuse_fence,
            self.device.present_queue(), // TODO: Don't need to pass this as param
            &[vk::PipelineStageFlags::BOTTOM_OF_PIPE], // TODO: Abstract later
            &[self.present_complete_semaphore],
            &[self.rendering_complete_semaphore],
        )?;

        let is_suboptimal = unsafe {
            self.swapchain.swapchain_loader.queue_present(
                self.device.present_queue(),
                &vk::PresentInfoKHR::builder()
                    .wait_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
                    .swapchains(std::slice::from_ref(&self.swapchain.swapchain))
                    .image_indices(std::slice::from_ref(&present_index))
                    .build(),
            )
        }?;
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
