pub mod context;
pub(crate) mod debug;
pub mod device;
pub mod resoruces;

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

pub struct InitData {
    pub backbuffer_resolution: Resolution,
}
