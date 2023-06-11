use resource_manager::ResourceId;

// TODO: Gotta figure out what to do here
pub type TaskFn<'a> = dyn FnMut() -> () + 'a;

pub struct Node<'a, T> {
    resources: Vec<ResourceId<T>>,
    task: Box<TaskFn<'a>>,
}

pub struct RenderGraphBuilder<'a, T> {
    nodes: Vec<Node<'a, T>>,
}
