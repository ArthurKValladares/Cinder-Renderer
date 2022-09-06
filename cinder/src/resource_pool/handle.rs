#[derive(Debug, Clone, Copy)]
pub struct Handle<T> {
    bits: u32,
    _marker: std::marker::PhantomData<T>,
}
