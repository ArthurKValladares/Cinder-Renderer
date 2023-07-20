use crate::command_queue::RenderAttachmentDesc;
use anyhow::Result;
use math::rect::Rect2D;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RenderAttachment {
    name: String,
    desc: RenderAttachmentDesc,
}

#[derive(Debug, Deserialize)]
pub struct RenderGraphPass {
    name: String,
    color_attachments: Vec<RenderAttachment>,
    depth_attachment: Option<RenderAttachment>,
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
