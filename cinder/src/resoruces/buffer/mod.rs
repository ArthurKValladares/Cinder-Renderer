use super::memory::{Memory, MemoryDescription};
pub use ash::vk;

#[derive(Debug, Clone, Copy)]
pub enum BufferUsage {
    Vertex,
    Index,
    TransferSrc,
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(usage: BufferUsage) -> Self {
        match usage {
            BufferUsage::Vertex => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::Index => vk::BufferUsageFlags::INDEX_BUFFER,
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
