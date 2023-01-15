use super::{
    memory::{Memory, MemoryType},
    sampler::Sampler,
};
use crate::{
    device::Device,
    util::{find_memory_type_index, MemoryMappablePointer},
};
use anyhow::Result;
use ash::vk;
use math::size::Size2D;
use rust_shader_tools::ReflectFormat;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
    #[error("Buffer is not mappable from CPU memory")]
    NotMemoryMappable,
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
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Format {
    R8_G8_B8_A8_Unorm,
    B8_G8_R8_A8_Unorm,
    D32_SFloat,
    D16Unorm,
    R32_G32_B32_A32_SFloat,
    R32_G32_B32_SFloat,
    R32_G32_SFloat,
    R32_SFloat,
}

impl Default for Format {
    fn default() -> Self {
        Self::R8_G8_B8_A8_Unorm
    }
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::R8_G8_B8_A8_Unorm => vk::Format::R8G8B8A8_UNORM,
            Format::B8_G8_R8_A8_Unorm => vk::Format::B8G8R8A8_UNORM,
            Format::D32_SFloat => vk::Format::D32_SFLOAT,
            Format::D16Unorm => vk::Format::D16_UNORM,
            Format::R32_G32_B32_A32_SFloat => vk::Format::R32G32B32A32_SFLOAT,
            Format::R32_G32_B32_SFloat => vk::Format::R32G32B32_SFLOAT,
            Format::R32_G32_SFloat => vk::Format::R32G32_SFLOAT,
            Format::R32_SFloat => vk::Format::R32_SFLOAT,
        }
    }
}

impl From<vk::Format> for Format {
    fn from(vk: vk::Format) -> Self {
        match vk {
            vk::Format::R8G8B8A8_UNORM => Self::R8_G8_B8_A8_Unorm,
            vk::Format::B8G8R8A8_UNORM => Self::B8_G8_R8_A8_Unorm,
            vk::Format::D32_SFLOAT => Self::D32_SFloat,
            vk::Format::D16_UNORM => Self::D16Unorm,
            vk::Format::R32G32B32A32_SFLOAT => Self::R32_G32_B32_A32_SFloat,
            vk::Format::R32G32B32_SFLOAT => Self::R32_G32_B32_SFloat,
            vk::Format::R32G32_SFLOAT => Self::R32_G32_SFloat,
            vk::Format::R32_SFLOAT => Self::R32_SFloat,
            _ => panic!("Unsupported image format: {vk:?}"),
        }
    }
}

// TODO: enum doesn't really work here, should be something more bitfield-like
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Usage {
    Depth,
    Texture,
    StorageTexture,
}

impl Default for Usage {
    fn default() -> Self {
        Self::Texture
    }
}

impl From<Usage> for vk::ImageUsageFlags {
    fn from(usage: Usage) -> Self {
        match usage {
            Usage::Depth => {
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC
            }
            Usage::Texture => vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            Usage::StorageTexture => vk::ImageUsageFlags::STORAGE,
        }
    }
}

impl From<Usage> for vk::ImageAspectFlags {
    fn from(usage: Usage) -> Self {
        match usage {
            Usage::Depth => vk::ImageAspectFlags::DEPTH,
            Usage::Texture => vk::ImageAspectFlags::COLOR,
            Usage::StorageTexture => vk::ImageAspectFlags::COLOR,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageDescription {
    pub format: Format,
    pub usage: Usage,
    pub memory_ty: MemoryType,
}

impl Default for ImageDescription {
    fn default() -> Self {
        Self {
            format: Default::default(),
            usage: Default::default(),
            memory_ty: MemoryType::GpuOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct ImageViewDescription {
    pub format: Format,
    pub usage: Usage,
}

pub struct Image {
    pub raw: vk::Image,
    pub size: Size2D<u32>,
    pub desc: ImageDescription,
    pub view: vk::ImageView,
    // TODO: Have memory live somewhere else, and manage it all together?
    pub memory: Memory,
    pub ptr: Option<MemoryMappablePointer>,
}

impl Image {
    pub fn create(device: &Device, size: Size2D<u32>, desc: ImageDescription) -> Result<Self> {
        let create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(desc.format.into())
            .extent(vk::Extent3D {
                width: size.width(),
                height: size.height(),
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        let image = unsafe { device.raw().create_image(&create_info, None) }?;
        let memory_req = unsafe { device.raw().get_image_memory_requirements(image) };
        let memory_index = find_memory_type_index(
            &memory_req,
            device.memopry_properties(),
            desc.memory_ty.into(),
        )
        .ok_or(ImageError::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_req.size,
            memory_type_index: memory_index,
            ..Default::default()
        };
        let memory = unsafe { device.raw().allocate_memory(&allocate_info, None) }?;
        unsafe {
            device.raw().bind_image_memory(image, memory, 0)?;
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
            .format(desc.format.into())
            .view_type(vk::ImageViewType::TYPE_2D);
        let view = unsafe { device.raw().create_image_view(&image_view_info, None) }?;

        let memory = Memory {
            raw: memory,
            req: memory_req,
        };

        let ptr = if desc.memory_ty.is_cpu_visible() {
            Some(memory.ptr(device.raw())?)
        } else {
            None
        };

        Ok(Image {
            raw: image,
            size,
            view,
            memory,
            desc,
            ptr,
        })
    }

    pub fn clean(&mut self, device: &Device) {
        unsafe {
            device.raw().destroy_image(self.raw, None);
            device.raw().destroy_image_view(self.view, None);
            self.memory.destroy(device.raw());
        }
    }

    pub fn dims(&self) -> Size2D<u32> {
        self.size
    }

    pub fn format(&self) -> Format {
        self.desc.format
    }

    pub fn mem_copy<T: Copy>(&self, offset: u64, data: &[T]) -> Result<(), ImageError> {
        self.ptr.map_or_else(
            || Err(ImageError::NotMemoryMappable),
            |ptr| {
                ptr.add(offset as usize).mem_copy(data);
                Ok(())
            },
        )
    }

    pub fn resize(&mut self, device: &Device, size: Size2D<u32>) -> Result<()> {
        self.clean(device);
        *self = Self::create(device, size, self.desc)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BindImageInfo {
    pub info: vk::DescriptorImageInfo,
    pub index: u32,
}

impl Image {
    pub fn bind_info(
        &self,
        sampler: &Sampler,
        index: u32, // TODO: This only makes sense for bindless
    ) -> BindImageInfo {
        BindImageInfo {
            info: vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // TODO: Should not be hard-coded?
                image_view: self.view,
                sampler: sampler.raw,
            },
            index,
        }
    }
}
