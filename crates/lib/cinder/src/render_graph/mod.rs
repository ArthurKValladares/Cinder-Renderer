use crate::{command_queue::RenderAttachment, device::Device};
use anyhow::Result;
use resource_manager::ResourceId;

pub struct RenderGraphNode {
    render_attachment: ResourceId<RenderAttachment>,
    inputs: Vec<ResourceId<RenderGraphResource>>,
    outputs: Vec<ResourceId<RenderGraphResource>>,
    edges: Vec<ResourceId<RenderGraphNode>>,
}

pub enum RenderGraphResourceType {
    Buffer,
    Texture,
    Attachment,
    Reference,
}

pub struct RenderGraphResource {
    ty: RenderGraphResourceType,
    producer: ResourceId<RenderGraphNode>,
    output: ResourceId<RenderGraphResource>,
    ref_count: usize,
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
