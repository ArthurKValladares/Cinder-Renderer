use super::ContextShared;
use crate::{
    device::Device,
    resoruces::{
        buffer::Buffer,
        texture::{self, Texture},
    },
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

    pub fn begin(&self, device: &Device) -> Result<()> {
        self.shared
            .begin(&device, device.setup_commands_reuse_fence)
    }

    pub fn end(&self, device: &Device) -> Result<()> {
        self.shared.end(device)
    }

    pub fn copy_buffer_to_texture(&self, device: &Device, buffer: &Buffer, texture: &Texture) {
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
