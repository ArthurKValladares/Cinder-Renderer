use super::{memory::Memory, sampler::Sampler};
use crate::util::find_memory_type_index;
use anyhow::Result;
use ash::vk;
use math::size::Size2D;
use rust_shader_tools::ReflectFormat;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageCreateError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}

pub fn reflect_format_to_vk(fmt: ReflectFormat, low_precision: bool) -> vk::Format {
    match (fmt, low_precision) {
        (ReflectFormat::Undefined, _) => vk::Format::UNDEFINED,
        (ReflectFormat::R32_UINT, _) => vk::Format::R32_UINT,
        (ReflectFormat::R32_SINT, _) => vk::Format::R32_SINT,
        (ReflectFormat::R32_SFLOAT, _) => vk::Format::R32_SFLOAT,
        (ReflectFormat::R32G32_UINT, _) => vk::Format::R32G32_UINT,
        (ReflectFormat::R32G32_SINT, _) => vk::Format::R32G32_SINT,
        (ReflectFormat::R32G32B32_UINT, _) => vk::Format::R32G32B32_UINT,
        (ReflectFormat::R32G32B32_SINT, _) => vk::Format::R32G32B32_SINT,
        (ReflectFormat::R32G32B32A32_UINT, _) => vk::Format::R32G32B32A32_UINT,
        (ReflectFormat::R32G32B32A32_SINT, _) => vk::Format::R32G32B32A32_SINT,
        (ReflectFormat::R32G32_SFLOAT, false) => vk::Format::R32G32_SFLOAT,
        (ReflectFormat::R32G32B32_SFLOAT, false) => vk::Format::R32G32B32_SFLOAT,
        (ReflectFormat::R32G32B32A32_SFLOAT, false) => vk::Format::R32G32B32A32_SFLOAT,
        (ReflectFormat::R32G32_SFLOAT, true) => vk::Format::R8G8_UNORM,
        (ReflectFormat::R32G32B32_SFLOAT, true) => vk::Format::R8G8B8_UNORM,
        (ReflectFormat::R32G32B32A32_SFLOAT, true) => vk::Format::R8G8B8A8_UNORM,
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Format {
    R8_G8_B8_A8_Unorm,
    D32_SFloat,
    D16Unorm,
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
            Format::D16Unorm => vk::Format::D16_UNORM,
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
            .format(desc.format.into())
            .extent(vk::Extent3D {
                width: desc.size.width(),
                height: desc.size.height(),
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

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
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(desc.usage.into())
                    .level_count(1)
                    .layer_count(1)
                    .build(),
            )
            .image(image)
            .format(create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D);

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

pub struct BindImageInfo {
    pub info: vk::DescriptorImageInfo,
    pub index: u32,
}

impl Image {
    pub fn bind_info(&self, sampler: &Sampler, index: u32) -> BindImageInfo {
        BindImageInfo {
            info: vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: self.view,
                sampler: sampler.raw,
            },
            index,
        }
    }
}
