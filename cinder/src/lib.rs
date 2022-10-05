use math::size::Size2D;

pub mod context;
pub mod device;
pub mod instance;
pub mod resoruces;
pub(crate) mod surface;
pub(crate) mod swapchain;
pub mod util;

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

pub struct InitData {
    pub backbuffer_resolution: Resolution,
}

impl From<Resolution> for Size2D<u32> {
    fn from(resolution: Resolution) -> Self {
        Size2D::new(resolution.width, resolution.height)
    }
}
