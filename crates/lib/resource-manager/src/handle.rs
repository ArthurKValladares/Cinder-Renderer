use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

#[repr(transparent)]
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

impl<T> Debug for ResourceHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceHandle")
            .field("id", &self.id)
            .finish()
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

impl<T> PartialEq for ResourceHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for ResourceHandle<T> {}

impl<T> Hash for ResourceHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialOrd for ResourceHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
impl<T> Ord for ResourceHandle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

unsafe impl<T> Send for ResourceHandle<T> {}
unsafe impl<T> Sync for ResourceHandle<T> {}
