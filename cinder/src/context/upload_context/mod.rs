use super::ContextShared;
use crate::{
    cinder::{self, Cinder},
    resoruces::{buffer::Buffer, image::Image},
};
use anyhow::Result;
use ash::vk;

pub struct UploadContextDescription {}

pub struct UploadContext {
    pub shared: ContextShared,
}

impl UploadContext {
    pub fn from_command_buffer(command_buffer: vk::CommandBuffer) -> Self {
        Self {
            shared: ContextShared::from_command_buffer(command_buffer),
        }
    }

    pub fn begin(&self, cinder: &Cinder) -> Result<()> {
        self.shared
            .begin(cinder.device(), cinder.setup_commands_reuse_fence)
    }

    pub fn end(
        &self,
        cinder: &Cinder,
        command_buffer_reuse_fence: vk::Fence,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
    ) -> Result<()> {
        self.shared.end(
            cinder.device(),
            command_buffer_reuse_fence,
            submit_queue,
            wait_mask,
            wait_semaphores,
            signal_semaphores,
        )
    }

    pub fn transition_depth_image(&self, cinder: &Cinder) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(cinder.depth_image.raw)
            .dst_access_mask(
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .layer_count(1)
                    .level_count(1)
                    .build(),
            )
            .build();

        unsafe {
            cinder.device().cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[layout_transition_barriers],
            )
        };
    }

    pub fn image_barrier_start(&self, cinder: &Cinder, image: &Image) {
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
            cinder.device().cmd_pipeline_barrier(
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

    pub fn image_barrier_end(&self, cinder: &Cinder, image: &Image) {
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
            cinder.device().cmd_pipeline_barrier(
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

    pub fn copy_buffer_to_image(&self, cinder: &Cinder, buffer: &Buffer, image: &Image) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: image.desc.size.width(),
                height: image.desc.size.height(),
                depth: 1,
            })
            .build();

        unsafe {
            cinder.device().cmd_copy_buffer_to_image(
                self.shared.command_buffer,
                buffer.raw,
                image.raw,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            )
        };
    }
}
