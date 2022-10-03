use super::memory::{Memory, MemoryDescription};
pub use ash::vk;

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
}

impl Buffer {
    pub fn num_bytes(&self) -> u32 {
        todo!()
    }
    pub fn stride(&self) -> u32 {
        todo!()
    }
    pub fn num_elements(&self) -> u32 {
        todo!()
    }
}

#[repr(transparent)]
pub struct BindBufferInfo(pub vk::DescriptorBufferInfo);

impl Buffer {
    pub fn bind_info(&self) -> BindBufferInfo {
        BindBufferInfo(vk::DescriptorBufferInfo {
            buffer: self.raw,
            offset: 0,
            range: self.size_bytes,
        })
    }
}
