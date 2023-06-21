use crate::{command_queue::RenderAttachment, device::Device};
use anyhow::Result;
use resource_manager::ResourceId;

pub struct RenderGraphNode<'a> {
    render_attachment: ResourceId<RenderAttachment>,
    inputs: Vec<ResourceId<RenderGraphResource<'a>>>,
    outputs: Vec<ResourceId<RenderGraphResource<'a>>>,
    edges: Vec<ResourceId<RenderGraphNode<'a>>>,
    name: &'a str,
}

pub enum RenderGraphResourceType {
    Buffer,
    Texture,
    Attachment,
    Reference,
}

pub struct RenderGraphResource<'a> {
    ty: RenderGraphResourceType,
    producer: ResourceId<RenderGraphNode<'a>>,
    output: ResourceId<RenderGraphResource<'a>>,
    ref_count: usize,
    name: &'a str,
}

#[derive(Debug, Default)]
pub struct RenderGraphBuilder {}

impl RenderGraphBuilder {
    pub fn build(device: &Device) -> Result<RenderGraph> {
        Ok(RenderGraph {})
    }
}

#[derive(Debug)]
pub struct RenderGraph {}

impl RenderGraph {
    pub fn render(&self, device: &Device) -> Result<()> {
        Ok(())
    }
}
