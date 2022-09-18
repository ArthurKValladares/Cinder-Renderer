use super::ContextShared;
use crate::{
    device::Device,
    resoruces::{buffer::Buffer, pipeline::GraphicsPipeline, render_pass::RenderPass},
};
use anyhow::Result;
use ash::vk::{self};

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

    pub fn set_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {}

    pub fn set_vertex_buffer(&self, buffer: Buffer) {}

    pub fn set_index_buffer(&self, buffer: Buffer) {}

    pub fn draw(&self) {}
}
