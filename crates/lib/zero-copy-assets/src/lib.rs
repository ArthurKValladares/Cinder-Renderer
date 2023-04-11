use std::path::Path;

use memmap2::Mmap;
use rkyv::{
    de::deserializers::SharedDeserializeMapError,
    ser::serializers::{AllocScratchError, CompositeSerializerError, SharedSerializeMapError},
    Archive, Deserialize, Serialize,
};
use thiserror::Error;
use zune_png::{error::PngDecodeErrors, PngDecoder};

// TODO: Figure a good number here
const SCRATCH_SPACE: usize = 4096;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Png decode error: {0:?}")]
    PngDecodeError(PngDecodeErrors),
    #[error("Invalid UTF-8 in path {0:?}")]
    InvalidUtf8(std::path::PathBuf),
    #[error(transparent)]
    DeserializeError(#[from] SharedDeserializeMapError),
    #[error(transparent)]
    SerializerError(
        #[from]
        CompositeSerializerError<
            std::convert::Infallible,
            AllocScratchError,
            SharedSerializeMapError,
        >,
    ),
}

#[repr(C)]
#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

impl ImageData {
    pub fn from_parts(width: u32, height: u32, bytes: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bytes,
        }
    }

    pub fn from_decoded_file(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }?;
        let ret = unsafe { rkyv::from_bytes_unchecked(&mmap) }?;
        Ok(ret)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), ImageError> {
        let path = path.as_ref();
        let bytes = rkyv::to_bytes::<_, SCRATCH_SPACE>(self)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn try_decoded_file(
        original_path: impl AsRef<Path>,
        decoded_path: impl AsRef<Path>,
    ) -> Result<Self, ImageError> {
        let original_path = original_path.as_ref();
        let decoded_path = decoded_path.as_ref();
        if decoded_path.exists() {
            Self::from_decoded_file(decoded_path)
        } else {
            let file_bytes = std::fs::read(original_path)?;
            let mut decoded_image = PngDecoder::new(&file_bytes);
            decoded_image
                .decode_headers()
                .map_err(ImageError::PngDecodeError)?;
            let (width, height) = decoded_image.get_dimensions().unwrap();
            let image_data = decoded_image
                .decode_raw()
                .map_err(ImageError::PngDecodeError)?;
            let ret = Self::from_parts(width as u32, height as u32, image_data);
            let parent = decoded_path
                .parent()
                .ok_or_else(|| ImageError::InvalidUtf8(decoded_path.to_owned()))?;
            std::fs::create_dir_all(parent)?;
            ret.write(decoded_path)?;
            Ok(ret)
        }
    }
}
