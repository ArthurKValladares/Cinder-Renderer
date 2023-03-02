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

#[derive(Debug)]
pub enum SizeClass {
    Absolute,
    SwapchainRelative,
    InputRelative,
}

#[derive(Debug)]
pub struct TextureInfo {
    size_class: SizeClass,
    size: Size3D<u32>,
    format: Format,
    samples: u32,
    levels: u32,
    layers: u32,
    persistent: bool,
}

#[derive(Debug)]
pub struct BufferInfo {
    size: usize,
    usage: BufferUsage,
    persistent: bool,
}

#[derive(Debug, Default)]
pub struct FrameGraph {}

impl FrameGraph {}
