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
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
    #[error("Buffer is not mappable from CPU memory")]
    NotMemoryMappable,
}

pub fn reflect_format_to_vk(fmt: ReflectFormat) -> vk::Format {
    match fmt {
        ReflectFormat::Undefined => vk::Format::UNDEFINED,
        ReflectFormat::R32_UINT => vk::Format::R32_UINT,
        ReflectFormat::R32_SINT => vk::Format::R32_SINT,
        ReflectFormat::R32_SFLOAT => vk::Format::R32_SFLOAT,
        ReflectFormat::R32G32_UINT => vk::Format::R32G32_UINT,
        ReflectFormat::R32G32_SINT => vk::Format::R32G32_SINT,
        ReflectFormat::R32G32_SFLOAT => vk::Format::R32G32_SFLOAT,
        ReflectFormat::R32G32B32_UINT => vk::Format::R32G32B32_UINT,
        ReflectFormat::R32G32B32_SINT => vk::Format::R32G32B32_SINT,
        ReflectFormat::R32G32B32_SFLOAT => vk::Format::R32G32B32_SFLOAT,
        ReflectFormat::R32G32B32A32_UINT => vk::Format::R32G32B32A32_UINT,
        ReflectFormat::R32G32B32A32_SINT => vk::Format::R32G32B32A32_SINT,
        ReflectFormat::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Format {
    R8G8B8A8_UNORM,
    R8G8B8A8_SRGB,
    B8G8R8A8_UNORM,
    D32_SFLOAT,
    D16_UNORM,
    R32G32B32A32_SFLOAT,
    R32G32B32_SFLOAT,
    R32G32_SFLOAT,
    R32_SFLOAT,
    R16G16B16A16_SFLOAT,
    R16G16_SFLOAT,
}

impl Default for Format {
    fn default() -> Self {
        Self::R8G8B8A8_UNORM
    }
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::R8G8B8A8_UNORM => vk::Format::R8G8B8A8_UNORM,
            Format::R8G8B8A8_SRGB => vk::Format::R8G8B8A8_SRGB,
            Format::B8G8R8A8_UNORM => vk::Format::B8G8R8A8_UNORM,
            Format::D32_SFLOAT => vk::Format::D32_SFLOAT,
            Format::D16_UNORM => vk::Format::D16_UNORM,
            Format::R32G32B32A32_SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
            Format::R32G32B32_SFLOAT => vk::Format::R32G32B32_SFLOAT,
            Format::R32G32_SFLOAT => vk::Format::R32G32_SFLOAT,
            Format::R32_SFLOAT => vk::Format::R32_SFLOAT,
            Format::R16G16B16A16_SFLOAT => vk::Format::R16G16B16A16_SFLOAT,
            Format::R16G16_SFLOAT => vk::Format::R16G16_SFLOAT,
        }
    }
}

impl From<vk::Format> for Format {
    fn from(vk: vk::Format) -> Self {
        match vk {
            vk::Format::R8G8B8A8_UNORM => Self::R8G8B8A8_UNORM,
            vk::Format::R8G8B8A8_SRGB => Self::R8G8B8A8_SRGB,
            vk::Format::B8G8R8A8_UNORM => Self::B8G8R8A8_UNORM,
            vk::Format::D32_SFLOAT => Self::D32_SFLOAT,
            vk::Format::D16_UNORM => Self::D16_UNORM,
            vk::Format::R32G32B32A32_SFLOAT => Self::R32G32B32A32_SFLOAT,
            vk::Format::R32G32B32_SFLOAT => Self::R32G32B32_SFLOAT,
            vk::Format::R32G32_SFLOAT => Self::R32G32_SFLOAT,
            vk::Format::R32_SFLOAT => Self::R32_SFLOAT,
            vk::Format::R16G16B16A16_SFLOAT => Self::R16G16B16A16_SFLOAT,
            vk::Format::R16G16_SFLOAT => Self::R16G16_SFLOAT,
            _ => panic!("Unsupported image format: {vk:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Layout {
    Undefined,
    General,
    ColorAttachment,
    DepthAttachment,
    Present,
    TransferDst,
    ShaderReadOnly,
    DepthStencilReadOnly,
}

impl Default for Layout {
    fn default() -> Self {
        Self::ColorAttachment
    }
}

impl From<Layout> for vk::ImageLayout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Undefined => vk::ImageLayout::UNDEFINED,
            Layout::General => vk::ImageLayout::GENERAL,
            Layout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            Layout::DepthAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            Layout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
            Layout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            Layout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            Layout::DepthStencilReadOnly => vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ImageUsage {
    Depth,
    DepthSampled,
    Texture,
    StorageTexture,
}

impl Default for ImageUsage {
    fn default() -> Self {
        Self::Texture
    }
}

impl ImageUsage {
    pub fn is_depth(&self) -> bool {
        matches!(self, ImageUsage::Depth | ImageUsage::DepthSampled)
    }
}

impl From<ImageUsage> for vk::ImageUsageFlags {
    fn from(usage: ImageUsage) -> Self {
        match usage {
            ImageUsage::Depth => vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            ImageUsage::DepthSampled => {
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED
            }
            ImageUsage::Texture => vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            ImageUsage::StorageTexture => vk::ImageUsageFlags::STORAGE,
        }
    }
}

impl From<ImageUsage> for vk::ImageAspectFlags {
    fn from(usage: ImageUsage) -> Self {
        match usage {
            ImageUsage::Depth | ImageUsage::DepthSampled => vk::ImageAspectFlags::DEPTH,
            ImageUsage::Texture => vk::ImageAspectFlags::COLOR,
            ImageUsage::StorageTexture => vk::ImageAspectFlags::COLOR,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageDescription {
    pub name: Option<&'static str>,
    pub format: Format,
    pub usage: ImageUsage,
    pub memory_ty: MemoryType,
}

impl Default for ImageDescription {
    fn default() -> Self {
        Self {
            name: None,
            format: Default::default(),
            usage: Default::default(),
            memory_ty: MemoryType::GpuOnly,
        }
    }
}

pub struct Image {
    pub raw: vk::Image,
    pub size: Size2D<u32>,
    pub desc: ImageDescription,
    pub view: vk::ImageView,
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

        if let Some(name) = desc.name {
            memory.set_name(device, name);
            device.set_name(vk::ObjectType::IMAGE, image, &format!("{name} [Image]"));
            device.set_name(
                vk::ObjectType::IMAGE_VIEW,
                view,
                &format!("{name} [Image View]"),
            );
        }

        Ok(Image {
            raw: image,
            size,
            view,
            memory,
            desc,
            ptr,
        })
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
        self.destroy(device);
        *self = Self::create(device, size, self.desc)?;
        Ok(())
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            device.raw().destroy_image(self.raw, None);
            device.raw().destroy_image_view(self.view, None);
            self.memory.destroy(device);
        }
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
        image_layout: Layout,
        index: Option<u32>,
    ) -> BindImageInfo {
        BindImageInfo {
            info: vk::DescriptorImageInfo {
                image_layout: image_layout.into(),
                image_view: self.view,
                sampler: sampler.raw,
            },
            index: index.unwrap_or(0),
        }
    }
}
