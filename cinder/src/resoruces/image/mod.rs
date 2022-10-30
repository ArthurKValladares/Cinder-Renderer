use super::{memory::Memory, sampler::Sampler};
use crate::util::find_memory_type_index;
use anyhow::Result;
use ash::vk;
use math::size::Size2D;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageCreateError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Format {
    R8_G8_B8_A8_Unorm,
    D32_SFloat,
    R32_G32_B32_A32_SFloat,
    R32_G32_B32_SFloat,
    R32_G32_SFloat,
    R32_SFloat,
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::R8_G8_B8_A8_Unorm => vk::Format::R8G8B8A8_UNORM,
            Format::D32_SFloat => vk::Format::D32_SFLOAT,
            Format::R32_G32_B32_A32_SFloat => vk::Format::R32G32B32A32_SFLOAT,
            Format::R32_G32_B32_SFloat => vk::Format::R32G32B32_SFLOAT,
            Format::R32_G32_SFloat => vk::Format::R32G32_SFLOAT,
            Format::R32_SFloat => vk::Format::R32_SFLOAT,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Usage {
    Depth,
    Texture,
}

impl From<Usage> for vk::ImageUsageFlags {
    fn from(usage: Usage) -> Self {
        match usage {
            Usage::Depth => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            Usage::Texture => vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        }
    }
}

impl From<Usage> for vk::ImageAspectFlags {
    fn from(usage: Usage) -> Self {
        match usage {
            Usage::Depth => vk::ImageAspectFlags::DEPTH,
            Usage::Texture => vk::ImageAspectFlags::COLOR,
        }
    }
}

pub struct ImageDescription {
    pub format: Format,
    pub usage: Usage,
    pub size: Size2D<u32>,
}

pub struct Image {
    pub raw: vk::Image,
    pub view: vk::ImageView,
    pub memory: Memory,
    pub desc: ImageDescription,
}

impl Image {
    pub(crate) fn create(
        device: &ash::Device,
        p_device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        desc: ImageDescription,
    ) -> Result<Self> {
        let create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: desc.size.width(),
                height: desc.size.height(),
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(desc.format.into())
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());
        let image = unsafe { device.create_image(&create_info, None) }?;
        let memory_req = unsafe { device.get_image_memory_requirements(image) };
        let memory_index = find_memory_type_index(
            &memory_req,
            p_device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or_else(|| ImageCreateError::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_req.size,
            memory_type_index: memory_index,
            ..Default::default()
        };
        let memory = unsafe { device.allocate_memory(&allocate_info, None) }?;
        unsafe {
            device.bind_image_memory(image, memory, 0)?;
        }

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(create_info.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: desc.usage.into(),
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let image_view = unsafe { device.create_image_view(&image_view_info, None) }?;

        let memory = Memory {
            raw: memory,
            req: memory_req,
        };

        Ok(Image {
            raw: image,
            view: image_view,
            memory,
            desc,
        })
    }

    pub(crate) fn clean(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_image(self.raw, None);
            device.destroy_image_view(self.view, None);
            self.memory.clean(device);
        }
    }

    pub fn dims(&self) -> Size2D<u32> {
        todo!()
    }
    pub fn format(&self) -> Format {
        todo!()
    }
}

pub struct BindImageInfo(pub vk::DescriptorImageInfo);

impl Image {
    pub fn bind_info(&self, sampler: &Sampler) -> BindImageInfo {
        BindImageInfo(vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: self.view,
            sampler: sampler.raw,
        })
    }
}
