use super::memory::{Memory, MemoryDescription};
use crate::{
    device::Device,
    util::{find_memory_type_index, MemoryMappablePointer},
};
use anyhow::Result;
pub use ash::vk;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BufferError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
    #[error("Buffer is not mappable from CPU memory")]
    NotMemoryMappable,
}

#[derive(Debug, Clone, Copy)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    TransferSrc,
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(usage: BufferUsage) -> Self {
        match usage {
            BufferUsage::Vertex => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::Index => vk::BufferUsageFlags::INDEX_BUFFER,
            BufferUsage::Uniform => vk::BufferUsageFlags::UNIFORM_BUFFER,
            BufferUsage::TransferSrc => vk::BufferUsageFlags::TRANSFER_SRC,
        }
    }
}

pub struct BufferDescription {
    pub size: u64,
    pub usage: BufferUsage,
    pub memory_desc: MemoryDescription,
}

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: Memory,
    pub size_bytes: u64,
    pub ptr: Option<MemoryMappablePointer>,
}

impl Buffer {
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn stride(&self) -> u32 {
        todo!()
    }

    pub fn num_elements(&self) -> u32 {
        todo!()
    }

    pub fn ptr(&self) -> Option<MemoryMappablePointer> {
        self.ptr
    }

    pub fn end_ptr(&self) -> Option<MemoryMappablePointer> {
        self.ptr.map(|ptr| ptr.add(self.size_bytes as usize))
    }

    pub fn mem_copy<T: Copy>(&self, data: &[T]) -> Result<(), BufferError> {
        self.ptr.map_or_else(
            || Err(BufferError::NotMemoryMappable),
            |ptr| {
                ptr.mem_copy(&data);
                Ok(())
            },
        )
    }
}

#[repr(transparent)]
pub struct BindBufferInfo(pub vk::DescriptorBufferInfo);

impl Buffer {
    pub(crate) fn create(device: &Device, desc: BufferDescription) -> Result<Self> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(desc.size)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_info, None) }?;
        let buffer_memory_req = unsafe { device.get_buffer_memory_requirements(buffer) };
        let buffer_memory_index = find_memory_type_index(
            &buffer_memory_req,
            device.memopry_properties(),
            desc.memory_desc.ty.clone().into(),
        )
        .ok_or_else(|| BufferError::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: buffer_memory_req.size,
            memory_type_index: buffer_memory_index,
            ..Default::default()
        };
        let buffer_memory = unsafe { device.allocate_memory(&allocate_info, None) }?;
        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0) }?;

        let memory = Memory {
            raw: buffer_memory,
            req: buffer_memory_req,
        };

        let ptr = if desc.memory_desc.is_cpu_visible() {
            unsafe {
                let ptr = device.map_memory(
                    memory.raw,
                    0,
                    buffer_memory_req.size,
                    vk::MemoryMapFlags::empty(),
                )?;
                Some(MemoryMappablePointer::from_raw_ptr(ptr))
            }
        } else {
            None
        };

        Ok(Buffer {
            raw: buffer,
            memory,
            size_bytes: desc.size,
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

    pub fn clean(&mut self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.raw, None);
            self.memory.clean(device);
        }
    }
}
