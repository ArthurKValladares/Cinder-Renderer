use crate::ResourceId;

enum Resource<T> {
    Alive { raw: T, generation: u32 },
    Dead,
}

impl<T> Resource<T> {
    fn is_alive(&self) -> bool {
        match self {
            Resource::Alive { .. } => true,
            Resource::Dead => false,
        }
    }
}

pub struct ResourcePool<T> {
    resources: Vec<Resource<T>>,
    generation: u32,
    free_indices: Vec<usize>,
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self {
            resources: Default::default(),
            generation: 0,
            free_indices: Default::default(),
        }
    }
}

impl<T> ResourcePool<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, resource: T) -> ResourceId<T> {
        let new_res = Resource::Alive {
            raw: resource,
            generation: self.generation,
        };
        match self.free_indices.pop() {
            Some(id) => {
                self.resources[id] = new_res;
                ResourceId::new(id, self.generation)
            }
            None => {
                let id = self.resources.len();
                self.resources.push(new_res);
                ResourceId::new(id, self.generation)
            }
        }
    }

    pub fn get(&self, handle: ResourceId<T>) -> Option<&T> {
        match self.resources.get(handle.id()) {
            Some(Resource::Alive { raw, generation }) if *generation == handle.generation() => {
                Some(raw)
            }
            _ => None,
        }
    }

    pub fn get_mut(&mut self, handle: ResourceId<T>) -> Option<&mut T> {
        match self.resources.get_mut(handle.id()) {
            Some(Resource::Alive { raw, generation }) if *generation == handle.generation() => {
                Some(raw)
            }
            _ => None,
        }
    }

    pub fn replace(&mut self, handle: ResourceId<T>, new: T) -> Option<T> {
        if handle.id() >= self.resources.len() {
            None
        } else {
            match self.resources[handle.id()] {
                Resource::Alive { raw: _, generation } if generation == handle.generation() => {
                    let old = std::mem::replace(
                        &mut self.resources[handle.id()],
                        Resource::Alive {
                            raw: new,
                            generation,
                        },
                    );
                    match old {
                        Resource::Alive { raw, .. } => Some(raw),
                        Resource::Dead => unreachable!(),
                    }
                }
                _ => None,
            }
        }
    }

    pub fn remove(&mut self, handle: ResourceId<T>) -> Option<T> {
        if handle.id() >= self.resources.len() {
            None
        } else {
            match self.resources[handle.id()] {
                Resource::Alive { raw: _, generation } if generation == handle.generation() => {
                    self.free_indices.push(handle.id());
                    self.generation += 1;
                    let old = std::mem::replace(&mut self.resources[handle.id()], Resource::Dead);
                    match old {
                        Resource::Alive { raw, .. } => Some(raw),
                        Resource::Dead => unreachable!(),
                    }
                }
                _ => None,
            }
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.generation += 1;
        self.free_indices.extend(
            self.resources
                .iter()
                .enumerate()
                .filter(|(_, res)| res.is_alive())
                .map(|(idx, _)| idx),
        );
        self.resources
            .drain(..)
            .filter(|res| res.is_alive())
            .map(|res| match res {
                Resource::Alive { raw, .. } => raw,
                Resource::Dead => unreachable!(),
            })
    }
}
