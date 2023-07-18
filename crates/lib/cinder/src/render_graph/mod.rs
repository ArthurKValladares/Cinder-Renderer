use crate::{command_queue::AttachmentLoadOp, resources::image::Format};
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderGraphResourceType {
    Buffer,
    Texture,
    Attachment,
    Reference,
    ShadingRate,
}

#[derive(Debug, Deserialize)]
pub struct BufferOutputInfo {
    size: u64,
    //usage: BufferUsage,
}

#[derive(Debug, Deserialize)]
pub struct TextureOuputInfo {
    resolution: [u32; 2],
    format: Format,
    load_op: AttachmentLoadOp,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceOutputInfo {
    external: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderGraphResourceInfo {
    Texture(TextureOuputInfo),
    Buffer(BufferOutputInfo),
    Reference(ReferenceOutputInfo),
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphOutput {
    name: String,
    #[serde(rename = "type")]
    ty: RenderGraphResourceType,
    info: RenderGraphResourceInfo,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphInput {
    name: String,
    #[serde(rename = "type")]
    ty: RenderGraphResourceType,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphPass {
    name: String,
    inputs: Vec<RenderGraphInput>,
    outputs: Vec<RenderGraphOutput>,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphData {
    name: String,
    passes: Vec<RenderGraphPass>,
}

impl RenderGraphData {
    pub fn from_json(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::open(&path)?;
        let result = serde_json::from_reader(file)?;
        Ok(result)
    }
}
