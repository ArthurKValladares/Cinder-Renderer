use anyhow::Result;
use memmap::MmapOptions;
use rkyv::{AlignedVec, Archive, Deserialize, Serialize};
use std::{fs::File, io::Write, path::Path};
use thiserror::Error;

// TODO: figure out N, use ScratchTracker.
pub const N: usize = 256;
pub const COMPILED_DIR: &str = "compiled_scenes";

#[derive(Debug, Error)]
pub enum CompiledSceneError {
    #[error("path did not have valid file name: {0:?}")]
    NoFileName(std::path::PathBuf),
    #[error("path contained invalid utf-8: {0:?}")]
    InvalidUtf8(std::path::PathBuf),
}

pub fn archive_to_file(bytes: AlignedVec, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let mut file = File::create(path).expect(&format!("Could not create file: {:?}", path));
    file.write_all(&bytes)?;
    Ok(())
}

#[macro_export]
macro_rules! from_archive_file {
    () => {
        pub fn from_archive_file(path: impl AsRef<Path>) -> Result<Self> {
            let file = File::open(path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            let archived = unsafe { rkyv::archived_root::<Self>(&mmap[..]) };
            let ret = archived.deserialize(&mut rkyv::Infallible)?;
            Ok(ret)
        }
    };
}

#[derive(Debug, Archive, Deserialize, Serialize)]
pub struct ImageBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImageBuffer {
    from_archive_file!();
}
