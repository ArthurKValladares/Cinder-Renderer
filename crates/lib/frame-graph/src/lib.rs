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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Attachment,
    Texture,
    Buffer,
    Reference,
}

#[derive(Debug, Deserialize)]
pub struct Resource {
    #[serde(rename = "type")]
    ty: ResourceType,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct NodeInfo {
    name: String,
    inputs: Vec<Resource>,
    outputs: Vec<Resource>,
}

#[derive(Debug, Deserialize)]
pub struct FrameGraphInfo {
    name: String,
    passes: Vec<NodeInfo>,
}

impl FrameGraphInfo {
    pub fn from_json(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let parser = serde_json::from_reader(reader)?;
        Ok(parser)
    }
}
