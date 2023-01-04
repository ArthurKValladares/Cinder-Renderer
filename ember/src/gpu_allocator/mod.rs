use anyhow::Result;
use cinder::{
    device::Device,
    resources::{
        buffer::{Buffer, BufferDescription, BufferUsage},
        memory::MemoryType,
    },
    util::align_size,
};

// TODO: The whole impl here is pretty imcomplete, should be a ring buffer and warn the user about overflow

const STAGING_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct BufferRegion {
    pub offset: u64,
    pub size: u64,
}

pub struct GpuStagingBuffer {
    buffer: Buffer,
    offset: u64,
}

impl GpuStagingBuffer {
    pub fn new(device: &Device, usage: BufferUsage, memory_ty: MemoryType) -> Result<Self> {
        let buffer = device.create_buffer(STAGING_BYTES, BufferDescription { usage, memory_ty })?;

        Ok(Self { buffer, offset: 0 })
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn copy_data<T: Copy>(&mut self, data: &[T]) -> Result<BufferRegion> {
        self.buffer.mem_copy(self.offset, data)?;
        let total_size = align_size(data);
        let region = BufferRegion {
            offset: self.offset,
            size: total_size,
        };
        self.offset += total_size;
        Ok(region)
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn available_bytes(&self) -> u64 {
        self.buffer.size_bytes - self.offset
    }
}
