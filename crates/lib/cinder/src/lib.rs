use math::size::Size2D;

pub mod cinder;
pub mod context;
pub mod device;
pub mod profiling;
pub mod resources;
pub mod util;
pub mod view;

pub use resource_manager::*;

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

pub struct InitData {
    pub backbuffer_resolution: Resolution,
    pub vsync: bool,
}

impl From<Resolution> for Size2D<u32> {
    fn from(resolution: Resolution) -> Self {
        Size2D::new(resolution.width, resolution.height)
    }
}
