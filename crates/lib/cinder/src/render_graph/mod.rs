use crate::command_queue::RenderAttachment;
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

pub struct RenderGraphBuilder {}

pub struct RenderGraph {}
