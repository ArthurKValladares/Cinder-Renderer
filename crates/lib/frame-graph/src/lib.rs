use cinder::resources::{
    buffer::BufferUsage,
    image::{Format, ImageUsage},
};
use math::size::Size3D;

#[derive(Debug, Copy, Clone)]
pub struct BufferInfo {
    size: usize,
    usage: BufferUsage,
}

#[derive(Debug, Copy, Clone)]
pub struct ImageInfo {
    size: Size3D,
    format: Format,
    usage: ImageUsage,
}

#[derive(Debug, Copy, Clone)]
pub enum Resource {
    Buffer(BufferInfo),
    Texture(ImageInfo),
    Attachment,
    Reference,
}

#[derive(Debug)]
pub struct Node {
    name: &'static str,
    inputs: Vec<Resource>,
    outputs: Vec<Resource>,
}
