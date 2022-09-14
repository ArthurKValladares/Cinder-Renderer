use super::Context;
use crate::{
    device::Device,
    resoruces::{buffer::Buffer, texture::Texture},
};
use anyhow::Result;

pub struct UploadContextDescription {}

pub struct UploadContext {}

impl Context for UploadContext {
    fn begin(&self, device: &Device) -> Result<()> {
        Ok(())
    }

    fn end(&self, device: &Device) -> Result<()> {
        Ok(())
    }

    fn resouce_barrier(&self, desc: super::BarrierDescription) {}
}

impl UploadContext {
    pub fn upload_buffer(&self, buffer: Buffer) {}

    pub fn upload_texture(&self, texture: Texture) {}
}
