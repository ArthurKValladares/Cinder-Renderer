use cinder::{
    resources::{
        buffer::BufferUsage,
        image::{Format, ImageUsage},
    },
    ResourceHandle,
};
use math::size::Size3D;

#[derive(Debug, Copy, Clone)]
pub struct BufferInfo {
    size: usize,
    usage: BufferUsage,
}

#[derive(Debug, Copy, Clone)]
pub struct ImageInfo {
    size: Size3D<u32>,
    format: Format,
    usage: ImageUsage,
}

#[derive(Debug, Copy, Clone)]
pub enum ResourceType {
    Buffer(BufferInfo),
    Texture(ImageInfo),
    Attachment,
    Reference,
}

#[derive(Debug)]
pub struct Resource {
    ty: ResourceType,
    producer: Option<ResourceHandle<Node>>,
    parent: Option<ResourceHandle<Resource>>,
    ref_count: usize,
}

#[derive(Debug)]
pub struct Node {
    name: &'static str,
    inputs: Vec<Resource>,
    outputs: Vec<Resource>,
    edges: Vec<ResourceHandle<Node>>,
}
