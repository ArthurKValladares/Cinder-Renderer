mod serde_repr;

use crate::{
    command_queue::RenderAttachment,
    device::Device,
    resources::{
        buffer::BufferUsage,
        image::{Format, ImageUsage},
    },
};
use anyhow::Result;
use math::size::Size3D;
use resource_manager::ResourceId;
// TODO: Temp
pub use serde_repr::RenderGraphRepr;

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

struct RenderGraphBuffer {}

pub struct BufferInfo {
    size: u64,
    usage: BufferUsage,
    id: ResourceId<RenderGraphBuffer>,
}

struct RenderGraphTexture {}

pub struct TextureInfo {
    size: Size3D<u32>,
    format: Format,
    usage: ImageUsage,
    id: ResourceId<RenderGraphTexture>,
}

pub struct ReferenceInfo<'a> {
    name: &'a str,
}

pub enum RenderGraphResourceInfo<'a> {
    Texture(TextureInfo),
    Buffer(BufferInfo),
    Reference(ReferenceInfo<'a>),
}

pub struct RenderGraphResource<'a> {
    ty: RenderGraphResourceType,
    info: RenderGraphResourceInfo<'a>,
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
