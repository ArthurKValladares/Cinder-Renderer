use ash::{vk, Device};

use crate::context::FrameNumber;

pub struct CommandBuffer {
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
}

impl CommandBuffer {
    pub fn raw(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    pub fn fence(&self) -> vk::Fence {
        self.fence
    }

    pub fn reset(&self, device: &Device) {
        unsafe {
            device
                .reset_command_buffer(
                    self.command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .unwrap()
        }
    }

    pub fn begin(&self, device: &Device) {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .unwrap()
        }
    }

    pub fn end(&self, device: &Device) {
        unsafe { device.end_command_buffer(self.command_buffer).unwrap() };
    }
}

pub struct CommandBufferPool {
    pub pool: vk::CommandPool,
    pub command_buffers: Vec<CommandBuffer>,
}

impl CommandBufferPool {
    pub fn new(
        device: &Device,
        queue_family_index: u32,
        num_command_buffers: u32,
    ) -> CommandBufferPool {
        unsafe {
            let pool_create_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let pool = device.create_command_pool(&pool_create_info, None).unwrap();

            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(num_command_buffers)
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap();

            let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

            let command_buffers: Vec<CommandBuffer> = command_buffers
                .iter()
                .map(|&command_buffer| {
                    let fence = device.create_fence(&fence_info, None).unwrap();
                    CommandBuffer {
                        command_buffer,
                        fence,
                    }
                })
                .collect();

            CommandBufferPool {
                pool,
                command_buffers,
            }
        }
    }

    pub fn get_command_buffer(&self, frame_number: FrameNumber) -> &CommandBuffer {
        &self.command_buffers[frame_number.raw() & self.command_buffers.len()]
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            for command_buffer in self.command_buffers.iter() {
                device.destroy_fence(command_buffer.fence, None);
            }

            device.destroy_command_pool(self.pool, None);
        }
    }
}
