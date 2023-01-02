use cinder::{
    device::Device,
    resources::image::{Format, Image, ImageDescription, ImageViewDescription, Usage},
};
use math::size::Size2D;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DepthPyramidError {
    #[error("could not resize depth pyramid")]
    ResizeError,
}

fn create_image_and_view(
    device: &Device,
    size: Size2D<u32>,
) -> anyhow::Result<(Image, ImageViewDescription)> {
    let format = Format::R32_SFloat;
    let usage = Usage::StorageTexture;
    let mut image = device.create_image(ImageDescription {
        format,
        usage,
        size,
    })?;
    let desc = ImageViewDescription { format, usage };
    image.add_view(device, desc)?;
    Ok((image, desc))
}

pub struct DepthPyramid {
    pub image: Image,
    pub image_desc: ImageViewDescription,
}

impl DepthPyramid {
    pub fn create(device: &Device, size: Size2D<u32>) -> anyhow::Result<Self> {
        let (image, image_desc) = create_image_and_view(device, size)?;
        Ok(Self { image, image_desc })
    }

    pub fn resize(&mut self, device: &Device, size: Size2D<u32>) -> Result<(), DepthPyramidError> {
        if let Ok((new_image, new_image_desc)) = create_image_and_view(device, size) {
            self.image.clean(device);
            self.image = new_image;
            self.image_desc = new_image_desc;
            Ok(())
        } else {
            Err(DepthPyramidError::ResizeError)
        }
    }

    pub fn clean(&mut self, device: &Device) {
        self.image.clean(device);
    }
}
