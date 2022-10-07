use ash::vk;
use std::ffi::c_void;

pub fn find_memory_type_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub unsafe fn mem_copy<T: Copy>(ptr: *mut c_void, data: &[T]) {
    let elem_size = std::mem::size_of::<T>() as vk::DeviceSize;
    let size = data.len() as vk::DeviceSize * elem_size;
    let mut align = ash::util::Align::new(ptr, elem_size, size);
    align.copy_from_slice(data);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryMappablePointer(*mut c_void);
unsafe impl Send for MemoryMappablePointer {}
unsafe impl Sync for MemoryMappablePointer {}

impl MemoryMappablePointer {
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    pub fn add(&self, count: usize) -> Self {
        Self(unsafe { self.0.add(count) })
    }

    pub fn mem_copy<T: Copy>(&self, data: &[T]) {
        unsafe { mem_copy(self.0, data) };
    }

    pub fn copy_from<T: Copy>(&self, data: &[T], size: usize) {
        unsafe { self.0.copy_from(data.as_ptr() as *mut c_void, size) };
    }
}
