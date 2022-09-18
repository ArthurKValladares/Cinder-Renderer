use super::ContextShared;
use crate::{
    device::Device,
    resoruces::{buffer::Buffer, pipeline::GraphicsPipeline, render_pass::RenderPass},
};
use anyhow::Result;
use ash::vk::{self};
use math::rect::Rect2D;

pub struct RenderContextDescription {}

pub struct RenderContext {
    pub shared: ContextShared,
}

impl RenderContext {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self {
            shared: ContextShared::from_command_buffer(command_buffer),
        }
    }

    pub fn begin(&self, device: &Device) -> Result<()> {
        self.shared.begin(&device, device.draw_commands_reuse_fence)
    }

    pub fn end(&self, device: &Device) -> Result<()> {
        self.shared.end(device)
    }

    pub fn begin_render_pass(&self, device: &Device, render_pass: &RenderPass, present_index: u32) {
        let create_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass.render_pass)
            .framebuffer(render_pass.framebuffers[present_index as usize])
            .render_area(render_pass.render_area)
            .clear_values(&render_pass.clear_values);

        unsafe {
            device.cmd_begin_render_pass(
                self.shared.command_buffer,
                &create_info,
                vk::SubpassContents::INLINE,
            )
        }
    }

    pub fn end_render_pass(&self, device: &Device, render_pass: &RenderPass) {
        unsafe { device.cmd_end_render_pass(self.shared.command_buffer) }
    }

    pub fn set_graphics_pipeline(&self, device: &Device, pipeline: &GraphicsPipeline) {
        unsafe {
            device.cmd_bind_pipeline(
                self.shared.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.common.pipeline,
            );
        }
    }

    pub fn set_vertex_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device.cmd_bind_vertex_buffers(self.shared.command_buffer, 0, &[buffer.raw], &[0])
        }
    }

    pub fn set_index_buffer(&self, device: &Device, buffer: &Buffer) {
        unsafe {
            device.cmd_bind_index_buffer(
                self.shared.command_buffer,
                buffer.raw,
                0,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn set_scissor(&self, device: &Device, rect: Rect2D<u32>) {
        unsafe {
            device.cmd_set_scissor(
                self.shared.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: rect.width(),
                        height: rect.height(),
                    },
                }],
            )
        }
    }

    pub fn set_viewport(&self, device: &Device, rect: Rect2D<u32>) {
        unsafe {
            device.cmd_set_viewport(
                self.shared.command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0 as f32,
                    width: rect.width() as f32,
                    height: rect.height() as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            )
        }
    }

    pub fn draw(&self) {}
}
