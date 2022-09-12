use super::Context;
use crate::resoruces::{buffer::Buffer, texture::Texture};

pub struct UploadContextDescription {}

pub struct UploadContext {}

impl Context for UploadContext {
    fn begin(&self) {}

    fn end(&self) {}

    fn resouce_barrier(&self, desc: super::BarrierDescription) {}
}

impl UploadContext {
    pub fn upload_buffer(&self, buffer: Buffer) {}

    pub fn upload_texture(&self, texture: Texture) {}
}
