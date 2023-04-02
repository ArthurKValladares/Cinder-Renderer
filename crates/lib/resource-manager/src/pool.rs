use std::collections::HashMap;

use crate::ResourceId;

pub struct ResourcePool<T> {
    // TODO: Better container for this, generational-arena
    resources: HashMap<usize, T>,
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
        }
    }
}

impl<T> ResourcePool<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, resource: T) -> ResourceId<T> {
        // Needs to guarantee unique id, so insert it not replacing an old resource
        let id = self.resources.len();
        self.resources.insert(id, resource);
        ResourceId::from_index(id)
    }

    pub fn replace(&mut self, handle: ResourceId<T>, new: T) -> Option<T> {
        self.resources.insert(handle.id(), new)
    }

    pub fn get(&self, handle: ResourceId<T>) -> Option<&T> {
        self.resources.get(&handle.id())
    }

    pub fn get_mut(&mut self, handle: ResourceId<T>) -> Option<&mut T> {
        self.resources.get_mut(&handle.id())
    }

    pub fn remove(&mut self, handle: ResourceId<T>) -> Option<T> {
        self.resources.remove(&handle.id())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.resources.drain().map(|(_, v)| v)
    }
}
