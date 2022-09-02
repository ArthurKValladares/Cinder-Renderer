mod backend;

use crate::backend::{Api, AsContext, BackendContext, ContextError};
use thiserror::Error;

pub struct Cinder {
    context: BackendContext,
}

pub enum PlatformData {
    Windows(()),
    MacOS(()),
}

pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8Srgb,
}

pub struct Resolution {
    pub format: TextureFormat,
    pub width: u32,
    pub height: u32,
}

pub struct InitData {
    pub debug_enabled: bool,
    pub profiling_enabled: bool,
    pub platform_data: PlatformData,
    pub backbuffer_resolution: Resolution,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewId(pub u32);

pub enum ColorClear {
    None,
    Value([u8; 4]),
}

pub enum DepthClear {
    None,
    Value(f32),
}

pub enum BackbufferRatio {
    Equal,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    Double,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameNumber(usize);

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum InitError {
    #[error(transparent)]
    Context(#[from] ContextError),
}

impl Api for Cinder {
    type Context = BackendContext;
}

impl Cinder {
    pub fn init(_init_data: InitData) -> Result<Self, InitError> {
        let context = BackendContext::init()?;
        Ok(Self { context })
    }

    pub fn set_view_color_clear(&mut self, id: ViewId, clear_op: ColorClear) {}
    pub fn set_view_depth_clear(&mut self, id: ViewId, clear_op: DepthClear) {}

    pub fn set_view_rect(&mut self, _id: ViewId, x: u32, y: u32, width: u32, height: u32) {}
    pub fn set_view_rect_relative_backbufer(
        &mut self,
        id: ViewId,
        x: u32,
        y: u32,
        backbuffer_ratio: BackbufferRatio,
    ) {
    }

    pub fn frame(&mut self) -> FrameNumber {
        FrameNumber(0)
    }
}
