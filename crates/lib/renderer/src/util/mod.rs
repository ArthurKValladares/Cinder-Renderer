use ash::vk;
use math::rect::Rect2D;
use num::ToPrimitive;
use std::ffi::c_void;

fn calc_padding(adr: vk::DeviceSize, align: vk::DeviceSize) -> vk::DeviceSize {
    (align - adr % align) % align
}

pub fn elem_size<T>(alignment: vk::DeviceSize) -> u64 {
    let padding = calc_padding(std::mem::size_of::<T>() as vk::DeviceSize, alignment);
    std::mem::size_of::<T>() as vk::DeviceSize + padding
}

pub fn align_size<T>(data: &[T]) -> u64 {
    let raw_elem_size = std::mem::size_of::<T>() as u64;
    let elem_size = elem_size::<T>(raw_elem_size as vk::DeviceSize);
    elem_size * data.len() as u64
}

pub fn find_memory_type_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: vk::PhysicalDeviceMemoryProperties,
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

/// # Safety
/// - pointer must not be null
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
    pub fn from_raw_ptr(ptr: *mut c_void) -> Self {
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

pub fn rect_to_vk<N: num::Num + Copy + ToPrimitive, M: num::Num + Copy + ToPrimitive>(
    rect: Rect2D<N, M>,
) -> Option<vk::Rect2D> {
    Some(vk::Rect2D {
        offset: vk::Offset2D {
            x: rect.offset().x().to_i32()?,
            y: rect.offset().y().to_i32()?,
        },
        extent: vk::Extent2D {
            width: rect.width().to_u32()?,
            height: rect.height().to_u32()?,
        },
    })
}
