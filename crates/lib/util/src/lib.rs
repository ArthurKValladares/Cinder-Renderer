pub use memoffset::offset_of;

pub fn size_of_slice<T>(slice: &[T]) -> u64 {
    (std::mem::size_of::<T>() * slice.len()) as u64
}

#[inline(always)]
pub fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    typed_to_bytes(std::slice::from_ref(p))
}

#[inline(always)]
pub fn typed_to_bytes<T: Sized>(typed: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            typed.as_ptr().cast(),
            typed.len() * std::mem::size_of::<T>(),
        )
    }
}
