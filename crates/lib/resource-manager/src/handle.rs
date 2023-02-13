use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct ResourceHandle<T> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T> ResourceHandle<T> {
    pub fn from_index(id: usize) -> Self {
        Self {
            id,
            _marker: Default::default(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn as_unit(&self) -> ResourceHandle<()> {
        ResourceHandle::<()>::from_index(self.id())
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
