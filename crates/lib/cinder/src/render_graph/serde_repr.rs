use super::RenderGraphResourceType;
use crate::resources::image::Format;
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct BufferOutputInfo {
    name: String,
    size: u64,
    //usage: BufferUsage,
}

#[derive(Debug, Deserialize)]
pub struct TextureOuputInfo {
    name: String,
    resolution: [u32; 2],
    format: Format,
    //usage: ImageUsage,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceOutputInfo {
    name: String,
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
    #[serde(rename = "type")]
    ty: RenderGraphResourceType,
    info: RenderGraphResourceInfo,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphInput {
    #[serde(rename = "type")]
    ty: RenderGraphResourceType,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphPass {
    name: String,
    inputs: Vec<RenderGraphInput>,
    outputs: Vec<RenderGraphOutput>,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphRepr {
    name: String,
    passes: Vec<RenderGraphPass>,
}

impl RenderGraphRepr {
    pub fn from_json(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::open(&path)?;
        let result = serde_json::from_reader(file)?;
        Ok(result)
    }
}
