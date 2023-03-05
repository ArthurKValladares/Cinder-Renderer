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
pub struct ImageInfo {
    pub size_class: SizeClass,
    pub size: Size3D<u32>,
    pub format: Format,
    pub persistent: bool,
}

#[derive(Debug)]
pub struct BufferInfo {
    pub size: usize,
    pub usage: BufferUsage,
    pub persistent: bool,
}

#[derive(Debug)]
pub enum PassResource {
    Image(ImageInfo),
    Buffer(BufferInfo),
}

#[derive(Debug)]
pub struct Pass<'a> {
    name: &'a str,
    inputs: Vec<PassResource>,
    outputs: Vec<PassResource>,
}

impl<'a> Pass<'a> {
    pub fn new<I>(name: &'a str, inputs: I, outputs: I) -> Self
    where
        I: IntoIterator<Item = PassResource>,
    {
        Self {
            name,
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
        }
    }
}

#[derive(Debug)]
pub struct FrameGraph<'a> {
    name: &'a str,
    passes: Vec<Pass<'a>>,
}

impl<'a> FrameGraph<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            passes: Default::default(),
        }
    }
}
