use std::path::Path;

use anyhow::Result;
use memmap2::Mmap;
use rkyv::{Archive, Deserialize, Serialize};

// TODO: Figure a good number here
const SCRATCH_SPACE: usize = 4096;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct ImageData {
    size: [u32; 2],
    bytes: Vec<u8>,
}

impl ImageData {
    pub fn from_parts(width: u32, height: u32, bytes: Vec<u8>) -> Self {
        Self {
            size: [width, height],
            bytes,
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file) }?;
        let ret = unsafe { rkyv::from_bytes_unchecked(&mmap) }?;
        Ok(ret)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let bytes = rkyv::to_bytes::<_, SCRATCH_SPACE>(self)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }
}
