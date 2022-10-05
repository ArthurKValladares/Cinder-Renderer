use super::ContextShared;
use crate::{
    cinder::Cinder,
    resoruces::{buffer::Buffer, texture::Texture},
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

    pub fn begin(&self, device: &Cinder) -> Result<()> {
        self.shared
            .begin(&device, device.setup_commands_reuse_fence)
    }

    pub fn end(&self, device: &Cinder) -> Result<()> {
        self.shared.end(device)
    }

    pub fn transition_depth_image(&self, device: &Cinder) {
        let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
            .image(device.depth_image.raw)
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
            device.cmd_pipeline_barrier(
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

    pub fn texture_barrier_start(&self, device: &Cinder, texture: &Texture) {
        let texture_barrier = vk::ImageMemoryBarrier {
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            image: texture.raw,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            device.cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[texture_barrier],
            )
        };
    }

    pub fn texture_barrier_end(&self, device: &Cinder, texture: &Texture) {
        let texture_barrier_end = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image: texture.raw,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            device.cmd_pipeline_barrier(
                self.shared.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[texture_barrier_end],
            )
        };
    }

    pub fn copy_buffer_to_texture(&self, device: &Cinder, buffer: &Buffer, texture: &Texture) {
        let buffer_copy_regions = vk::BufferImageCopy::builder()
            .image_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .build(),
            )
            .image_extent(vk::Extent3D {
                width: texture.desc.size.width(),
                height: texture.desc.size.height(),
                depth: 1,
            })
            .build();

        unsafe {
            device.cmd_copy_buffer_to_image(
                self.shared.command_buffer,
                buffer.raw,
                texture.raw,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            )
        };
    }
}
