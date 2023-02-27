use anyhow::Result;
use cinder::{
    context::render_context::{AttachmentLoadOp, ClearValue},
    resources::{
        buffer::BufferUsage,
        image::{Format, ImageUsage},
    },
    ResourceHandle,
};
use math::size::Size3D;
use serde::Deserialize;
use std::{fs::File, io::BufReader, path::Path};

#[derive(Deserialize, Debug)]
pub struct BufferInfo {
    size: usize,
    usage: BufferUsage,
}

#[derive(Deserialize, Debug)]
pub struct ImageInfo {
    size: Size3D<u32>,
    scale: [f32; 2],
    format: Format,
    usage: ImageUsage,
    clear_value: ClearValue,
    load_op: AttachmentLoadOp,
}

#[derive(Deserialize, Debug)]
pub enum ResourceInfo {
    Buffer(BufferInfo),
    Texture(ImageInfo),
    Attachment,
    Reference,
}

#[derive(Debug)]
pub struct Resource {
    info: ResourceInfo,
    producer: Option<ResourceHandle<Node>>,
    parent: Option<ResourceHandle<Resource>>,
    ref_count: usize,
}

#[derive(Debug)]
pub struct Node {
    name: String,
    inputs: Vec<Resource>,
    outputs: Vec<Resource>,
    edges: Vec<ResourceHandle<Node>>,
}

#[derive(Debug)]
pub struct FrameGraph {
    name: String,
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]

pub struct FrameGraphParser {
    name: String,
    node_data: Vec<ResourceInfo>,
}

impl FrameGraphParser {
    pub fn from_json(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let parser = serde_json::from_reader(reader)?;
        Ok(parser)
    }
}
