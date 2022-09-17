pub mod graphics_context;
pub mod upload_context;

use crate::device::Device;
use anyhow::Result;
use ash::vk;

pub struct BarrierDescription {}

pub struct ContextShared {
    pub command_buffer: vk::CommandBuffer,
}

impl ContextShared {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self { command_buffer }
    }
}

impl ContextShared {
    fn begin(&self, device: &ash::Device, command_buffer_reuse_fence: vk::Fence) -> Result<()> {
        unsafe { device.wait_for_fences(&[command_buffer_reuse_fence], true, std::u64::MAX) }?;

        unsafe { device.reset_fences(&[command_buffer_reuse_fence]) }?;

        unsafe {
            device.reset_command_buffer(
                self.command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
        }?;

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(self.command_buffer, &command_buffer_begin_info) }?;

        Ok(())
    }

    fn end(&self, device: &ash::Device) -> Result<()> {
        unsafe { device.end_command_buffer(self.command_buffer) }?;

        Ok(())
    }
}
