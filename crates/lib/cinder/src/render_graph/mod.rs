use crate::device::Device;
use anyhow::Result;
use resource_manager::ResourceId;

pub struct Task<T, F: FnMut(&Device) -> Result<()>> {
    resources: Vec<ResourceId<T>>,
    task: F,
}

pub type NodeFn<'a> = dyn FnMut(&Device) -> Result<()> + 'a;
pub struct Node<'a, T> {
    resources: Vec<ResourceId<T>>,
    task: Box<NodeFn<'a>>,
}

pub struct RenderGraphBuilder<'a, T> {
    nodes: Vec<Node<'a, T>>,
}

impl<'a, T> RenderGraphBuilder<'a, T> {
    pub fn with_task<'b: 'a, F: FnMut(&Device) -> Result<()> + 'b>(
        mut self,
        task: Task<T, F>,
    ) -> Self {
        let Task { resources, task } = task;

        self.nodes.push(Node {
            resources,
            task: Box::new(task),
        });

        self
    }
}
