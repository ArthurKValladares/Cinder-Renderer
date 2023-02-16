use std::collections::HashMap;

use crate::ResourceHandle;

pub struct ResourcePool<T> {
    // TODO: Better container for this, generational-arena
    resources: HashMap<usize, T>,
    to_be_freed: Vec<T>,
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            to_be_freed: Default::default(),
        }
    }
}

impl<T> ResourcePool<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, resource: T) -> ResourceHandle<T> {
        let id = self.resources.len();
        self.resources.insert(id, resource);
        ResourceHandle::from_index(id)
    }

    // TODO: Figure out a more final API later
    pub fn replace(&mut self, handle: ResourceHandle<T>, new: T) {
        if let Some(old) = self.resources.insert(handle.id(), new) {
            self.to_be_freed.push(old);
        }
    }

    pub fn get(&self, handle: ResourceHandle<T>) -> Option<&T> {
        self.resources.get(&handle.id())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.resources.drain().map(|(_, v)| v)
    }
}
