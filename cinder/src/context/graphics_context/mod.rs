use super::Context;
use crate::{
    device::Device,
    resoruces::{buffer::Buffer, pipeline::GraphicsPipeline},
};
use anyhow::Result;
use ash::vk;

pub struct GraphicsContextDescription {}

pub struct GraphicsContext {
    command_buffer: vk::CommandBuffer,
}

impl GraphicsContext {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self { command_buffer }
    }
}

impl Context for GraphicsContext {
    fn begin(&self, device: &Device) -> Result<()> {
        unsafe {
            device.begin_command_buffer(
                self.command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?
        };

        Ok(())
    }

    fn end(&self, device: &Device) -> Result<()> {
        unsafe { device.end_command_buffer(self.command_buffer)? };

        Ok(())
    }

    fn resouce_barrier(&self, desc: super::BarrierDescription) {}
}

impl GraphicsContext {
    pub fn set_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {}

    pub fn set_vertex_buffer(&self, buffer: Buffer) {}

    pub fn set_index_buffer(&self, buffer: Buffer) {}

    pub fn draw(&self) {}
}
