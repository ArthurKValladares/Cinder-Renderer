use crate::ResourceHandle;

pub struct ResourcePool<T> {
    // TODO: Better container for this, generational-arena
    resources: Vec<T>,
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

    pub fn insert(&mut self, resource: T) -> ResourceHandle<T> {
        let id = self.resources.len();
        self.resources.push(resource);
        ResourceHandle::from_index(id)
    }

    pub fn get(&self, handle: ResourceHandle<T>) -> Option<&T> {
        // TODO: This won't be a vec in the future
        if handle.id() >= self.resources.len() {
            None
        } else {
            Some(&self.resources[handle.id()])
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.resources.drain(..)
    }
}
