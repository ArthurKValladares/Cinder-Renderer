use cinder::{
    device::Device,
    resoruces::image::{Format, Image, ImageDescription, ImageViewDescription, Usage},
};
use math::size::Size2D;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DepthPyramidError {
    #[error("could not resize depth pyramid")]
    ResizeError,
}

fn create_image_and_view(device: &Device, size: Size2D<u32>) -> anyhow::Result<Image> {
    let format = Format::R32_SFloat;
    let usage = Usage::StorageTexture;
    let mut image = device.create_image(ImageDescription {
        format,
        usage,
        size,
    })?;
    image.add_view(device, ImageViewDescription { format, usage })?;
    Ok(image)
}

pub struct DepthPyramid {
    image: Image,
}

impl DepthPyramid {
    pub fn create(device: &Device, size: Size2D<u32>) -> anyhow::Result<Self> {
        let image = create_image_and_view(device, size)?;
        Ok(Self { image })
    }

    pub fn resize(&mut self, device: &Device, size: Size2D<u32>) -> Result<(), DepthPyramidError> {
        if let Ok(new_image) = create_image_and_view(device, size) {
            self.image.clean(device);
            self.image = new_image;
            Ok(())
        } else {
            Err(DepthPyramidError::ResizeError)
        }
    }

    pub fn clean(&mut self, device: &Device) {
        self.image.clean(device);
    }
}
