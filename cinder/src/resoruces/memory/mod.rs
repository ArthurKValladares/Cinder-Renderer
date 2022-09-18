use ash::vk;

pub enum MemoryType {
    CpuVisible,
    GpuOnly,
}

pub struct MemoryDescription {
    pub ty: MemoryType,
}

impl From<MemoryType> for vk::MemoryPropertyFlags {
    fn from(ty: MemoryType) -> Self {
        match ty {
            MemoryType::CpuVisible => {
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT
            }
            MemoryType::GpuOnly => vk::MemoryPropertyFlags::DEVICE_LOCAL,
        }
    }
}

pub struct Memory {
    pub raw: vk::DeviceMemory,
    pub req: vk::MemoryRequirements,
}
