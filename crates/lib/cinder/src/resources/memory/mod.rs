use crate::{device::Device, util::MemoryMappablePointer};
use anyhow::Result;
use ash::vk;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum MemoryType {
    CpuVisible,
    GpuOnly,
}

impl Default for MemoryType {
    fn default() -> Self {
        Self::CpuVisible
    }
}

impl MemoryType {
    pub fn is_cpu_visible(&self) -> bool {
        *self == Self::CpuVisible
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
    pub fn ptr(&self, device: &ash::Device) -> Result<MemoryMappablePointer> {
        unsafe {
            let ptr = device.map_memory(self.raw, 0, self.req.size, vk::MemoryMapFlags::empty())?;
            Ok(MemoryMappablePointer::from_raw_ptr(ptr))
        }
    }

    // TODO: set_name function standard way to do this
    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(
            vk::ObjectType::DEVICE_MEMORY,
            self.raw,
            &format!("{name} [device memory]"),
        );
    }

    pub(crate) fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.free_memory(self.raw, None);
        }
    }
}
