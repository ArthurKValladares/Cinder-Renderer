use std::path::Path;

use memmap2::Mmap;
pub use rkyv;
use rkyv::{
    de::deserializers::{SharedDeserializeMap, SharedDeserializeMapError},
    ser::serializers::{
        AllocScratchError, AllocSerializer, CompositeSerializerError, SharedSerializeMapError,
    },
    Archive, Deserialize, Serialize,
};
use thiserror::Error;

const SCRATCH_SPACE: usize = 4096;

#[derive(Debug, Error)]
pub enum ZeroCopyError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
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
    #[error("{0:?}")]
    Fallback(String),
}

pub trait LoadFromPath: Sized {
    fn from_resource_path(path: impl AsRef<Path>) -> Result<Self, ZeroCopyError>;
}

impl LoadFromPath for ImageData {
    fn from_resource_path(path: impl AsRef<Path>) -> Result<Self, ZeroCopyError> {
        let path = path.as_ref();
        let file_bytes = std::fs::read(path)?;
        let image = image::load_from_memory(&file_bytes)
            .map_err(|err| ZeroCopyError::Fallback(err.to_string()))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_data = image.into_raw();
        Ok(Self::from_parts(width, height, image_data))
    }
}

pub fn from_decoded_file<T>(path: impl AsRef<Path>) -> Result<T, ZeroCopyError>
where
    T: Archive,
    T::Archived: Deserialize<T, SharedDeserializeMap>,
{
    let path = path.as_ref();
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { Mmap::map(&file) }?;
    let ret = unsafe { rkyv::from_bytes_unchecked(&mmap) }?;
    Ok(ret)
}

pub fn write<T>(resource: &T, path: impl AsRef<Path>) -> Result<(), ZeroCopyError>
where
    T: Serialize<AllocSerializer<SCRATCH_SPACE>>,
{
    let path = path.as_ref();
    let bytes = rkyv::to_bytes::<_, SCRATCH_SPACE>(resource)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn try_decoded_file<T>(
    original_path: impl AsRef<Path>,
    decoded_path: impl AsRef<Path>,
) -> Result<T, ZeroCopyError>
where
    T: Archive + Serialize<AllocSerializer<SCRATCH_SPACE>> + LoadFromPath,
    T::Archived: Deserialize<T, SharedDeserializeMap>,
{
    let original_path = original_path.as_ref();
    let decoded_path = decoded_path.as_ref();
    if decoded_path.exists() {
        from_decoded_file(decoded_path)
    } else {
        let ret = T::from_resource_path(original_path)?;
        let parent = decoded_path
            .parent()
            .ok_or_else(|| ZeroCopyError::InvalidUtf8(decoded_path.to_owned()))?;
        std::fs::create_dir_all(parent)?;
        write(&ret, decoded_path)?;
        Ok(ret)
    }
}

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
}
