use super::memory::{Memory, MemoryType};
use crate::{
    device::Device,
    util::{find_memory_type_index, MemoryMappablePointer},
};
use anyhow::Result;
pub use ash::vk;
use bitflags::bitflags;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
    #[error("Buffer is not mappable from CPU memory")]
    NotMemoryMappable,
}

bitflags! {
    #[derive(Debug, Default, serde::Deserialize, Copy, Clone)]
    pub struct BufferUsage: u32 {
        const VERTEX = 0x00000080;
        const INDEX = 0x00000040;
        const UNIFORM = 0x00000010;
        const STORAGE = 0x00000020;
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
    }
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(value: BufferUsage) -> Self {
        vk::BufferUsageFlags::from_raw(value.bits())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BufferDescription {
    pub name: Option<&'static str>,
    pub usage: BufferUsage,
    pub memory_ty: MemoryType,
}

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: Memory,
    pub size_bytes: u64,
    pub num_elements: Option<u32>,
    pub ptr: Option<MemoryMappablePointer>,
}

#[repr(transparent)]
#[derive(Debug)]
pub struct BindBufferInfo(pub vk::DescriptorBufferInfo);

impl Buffer {
    pub(crate) fn create(device: &Device, size: u64, desc: BufferDescription) -> Result<Self> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.raw().create_buffer(&buffer_info, None) }?;
        let buffer_memory_req = unsafe { device.raw().get_buffer_memory_requirements(buffer) };
        let buffer_memory_index = find_memory_type_index(
            &buffer_memory_req,
            device.memopry_properties(),
            desc.memory_ty.into(),
        )
        .ok_or(BufferError::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: buffer_memory_req.size,
            memory_type_index: buffer_memory_index,
            ..Default::default()
        };
        let buffer_memory = unsafe { device.raw().allocate_memory(&allocate_info, None) }?;
        unsafe { device.raw().bind_buffer_memory(buffer, buffer_memory, 0) }?;

        let memory = Memory {
            raw: buffer_memory,
            req: buffer_memory_req,
        };

        let ptr = if desc.memory_ty.is_cpu_visible() {
            Some(memory.ptr(device.raw())?)
        } else {
            None
        };

        if let Some(name) = desc.name {
            memory.set_name(device, name);
            device.set_name(vk::ObjectType::BUFFER, buffer, &format!("{name} [Buffer]"));
        }

        Ok(Buffer {
            raw: buffer,
            memory,
            size_bytes: size,
            num_elements: None,
            ptr,
        })
    }

    pub fn bind_info(&self) -> BindBufferInfo {
        BindBufferInfo(vk::DescriptorBufferInfo {
            buffer: self.raw,
            offset: 0,
            range: self.size_bytes,
        })
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn num_elements(&self) -> Option<u32> {
        self.num_elements
    }

    pub fn ptr(&self) -> Option<MemoryMappablePointer> {
        self.ptr
    }

    pub fn end_ptr(&self) -> Option<MemoryMappablePointer> {
        self.ptr.map(|ptr| ptr.add(self.size_bytes() as usize))
    }

    pub fn mem_copy<T: Copy>(&self, offset: u64, data: &[T]) -> Result<(), BufferError> {
        self.ptr.map_or_else(
            || Err(BufferError::NotMemoryMappable),
            |ptr| {
                ptr.add(offset as usize).mem_copy(data);
                Ok(())
            },
        )
    }

    pub fn destroy(&self, device: &Device) {
        unsafe {
            device.raw().destroy_buffer(self.raw, None);
            self.memory.destroy(device);
        }
    }
}
