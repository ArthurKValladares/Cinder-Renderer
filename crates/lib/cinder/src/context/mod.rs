pub mod render_context;
pub mod upload_context;

use anyhow::Result;
use ash::vk;

use crate::device::Device;

// TODO: This separate context thing is not good

pub struct BarrierDescription {}

pub struct ContextShared {
    pub command_buffer: vk::CommandBuffer,
}

impl ContextShared {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self { command_buffer }
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(
            vk::ObjectType::COMMAND_BUFFER,
            self.command_buffer,
            &format!("{} [command buffer]", name),
        );
    }
}

impl ContextShared {
    pub fn begin(&self, device: &ash::Device, command_buffer_reuse_fence: vk::Fence) -> Result<()> {
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

    pub fn end(
        &self,
        device: &ash::Device,
        command_buffer_reuse_fence: vk::Fence,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
    ) -> Result<()> {
        unsafe { device.end_command_buffer(self.command_buffer) }?;

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(std::slice::from_ref(&self.command_buffer))
            .signal_semaphores(signal_semaphores)
            .build();

        unsafe { device.queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence) }?;

        Ok(())
    }
}
