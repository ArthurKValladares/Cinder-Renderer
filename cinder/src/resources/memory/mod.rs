use ash::vk;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum MemoryType {
    CpuVisible,
    GpuOnly,
}

pub struct MemoryDescription {
    pub ty: MemoryType,
}

impl MemoryDescription {
    pub fn is_cpu_visible(&self) -> bool {
        self.ty == MemoryType::CpuVisible
    }
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

impl Memory {
    pub(crate) fn clean(&mut self, device: &ash::Device) {
        unsafe {
            device.free_memory(self.raw, None);
        }
    }
}