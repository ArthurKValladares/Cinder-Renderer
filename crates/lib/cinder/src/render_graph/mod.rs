use crate::{
    device::Device,
    resources::{
        buffer::Buffer,
        image::{Image, ImageUsage, Layout},
    },
};
use anyhow::Result;

pub struct ImageResource<T> {
    f: Box<dyn Fn(&Device, &mut T) -> Image>,
    aspect_mask: ImageUsage,
    old_layout: Layout,
    new_layout: Layout,
}

pub struct BufferResource<T> {
    f: Box<dyn Fn(&Device, &mut T) -> Buffer>,
    // TODO: Buffer memory barrier stuff
}

pub enum Resource<T> {
    Buffer(BufferResource<T>),
    Image(ImageResource<T>),
}

pub struct Task<T, F: FnMut(&Device) -> Result<()>> {
    resources: Vec<Resource<T>>,
    task: F,
}

pub type NodeFn<'a> = dyn FnMut(&Device) -> Result<()> + 'a;
pub struct Node<'a, T> {
    resources: Vec<Resource<T>>,
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
