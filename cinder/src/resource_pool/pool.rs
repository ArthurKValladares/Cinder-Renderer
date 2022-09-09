use crate::resource_pool::Handle;
use std::collections::HashMap;

pub struct Pool<T> {
    map: HashMap<Handle<T>, T>,
}
