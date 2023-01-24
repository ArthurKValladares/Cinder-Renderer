use super::ContextShared;
use crate::{
    device::Device,
    resources::{buffer::Buffer, image::Image},
};
use anyhow::Result;
use ash::vk;

#[derive(Debug, Clone, Copy, Default)]
pub struct UploadContextDescription {
    pub name: Option<&'static str>,
}

pub struct UploadContext {
    pub shared: ContextShared,
}

impl UploadContext {
    pub fn new(device: &Device, desc: UploadContextDescription) -> Result<Self> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(device.command_pool())
            .level(vk::CommandBufferLevel::PRIMARY);

        let shared = ContextShared::from_command_buffer(
            unsafe {
                device
                    .raw()
                    .allocate_command_buffers(&command_buffer_allocate_info)?
            }[0],
        );

        if let Some(name) = desc.name {
            shared.set_name(device, name);
        }

        Ok(Self { shared })
    }

    pub fn begin(&self, device: &Device, fence: ash::vk::Fence) -> Result<()> {
        self.shared.begin(device, fence)
    }

    pub fn end(
        &self,
        device: &Device,
        command_buffer_reuse_fence: vk::Fence,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
    ) -> Result<()> {
        self.shared.end(
            device,
            command_buffer_reuse_fence,
            submit_queue,
            wait_mask,
            wait_semaphores,
            signal_semaphores,
        )
    }

    pub fn image_barrier_start(&self, device: &Device, image: &Image) {
        let image_barrier = vk::ImageMemoryBarrier {
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            image: image.raw,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            device.raw().cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier],
            )
        };
    }

    pub fn image_barrier_end(&self, device: &Device, image: &Image) {
        let image_barrier_end = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image: image.raw,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            device.raw().cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier_end],
            )
        };
    }

    pub fn copy_buffer_to_image(&self, device: &Device, buffer: &Buffer, image: &Image) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(image.desc.usage.into())
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: image.size.width(),
                height: image.size.height(),
                depth: 1,
            })
            .build();

        unsafe {
            device.raw().cmd_copy_buffer_to_image(
                self.shared.command_buffer,
                buffer.raw,
                image.raw,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            )
        };
    }

    pub fn copy_buffer(
        &self,
        device: &Device,
        src: &Buffer,
        dst: &Buffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) {
        unsafe {
            device.raw().cmd_copy_buffer(
                self.shared.command_buffer,
                src.raw,
                dst.raw,
                &[vk::BufferCopy {
                    src_offset,
                    dst_offset,
                    size,
                }],
            );
        }
    }
}
