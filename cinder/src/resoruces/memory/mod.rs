use ash::vk;

pub enum MemoryType {
    CpuVisible,
    GpuOnly,
}

pub struct MemoryDescription {
    pub ty: MemoryType,
}

pub struct Memory {
    pub raw: vk::DeviceMemory,
}
