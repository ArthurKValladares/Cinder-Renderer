use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RenderGraphPass {
    name: String,
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
