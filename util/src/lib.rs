#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            (&b.$field as *const _ as isize) - (&b as *const _ as isize)
        }
    }};
}

pub fn size_of_slice<T>(slice: &[T]) -> u64 {
    (std::mem::size_of::<T>() * slice.len()) as u64
}

pub fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>()) }
}
