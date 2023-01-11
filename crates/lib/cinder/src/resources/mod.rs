use std::marker::PhantomData;

pub mod bind_group;
pub mod buffer;
pub mod image;
pub mod memory;
pub mod pipeline;
pub mod sampler;
pub mod shader;

#[repr(transparent)]
pub struct ResourceHandle<T> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T> ResourceHandle<T> {
    pub(crate) fn from_index(id: usize) -> Self {
        Self {
            id,
            _marker: Default::default(),
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }
}

impl<T> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: Default::default(),
        }
    }
}

impl<T> Copy for ResourceHandle<T> {}
