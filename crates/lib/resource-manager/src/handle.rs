use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

#[repr(transparent)]
pub struct ResourceId<T> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T> ResourceId<T> {
    pub fn from_index(id: usize) -> Self {
        Self {
            id,
            _marker: Default::default(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

impl<T> Debug for ResourceId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceId").field("id", &self.id).finish()
    }
}

impl<T> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _marker: Default::default(),
        }
    }
}
impl<T> Copy for ResourceId<T> {}

impl<T> PartialEq for ResourceId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for ResourceId<T> {}

impl<T> Hash for ResourceId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> PartialOrd for ResourceId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
impl<T> Ord for ResourceId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

unsafe impl<T> Send for ResourceId<T> {}
unsafe impl<T> Sync for ResourceId<T> {}
