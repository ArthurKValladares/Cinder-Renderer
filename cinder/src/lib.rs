mod backend;
pub mod context;
pub mod init;
pub mod view;

use crate::{
    context::{Context, ContextError},
    init::InitData,
    view::{Clear, ColorClear, DepthClear, ViewId},
};
use thiserror::Error;

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

pub struct Cinder {
    context: Context,
}

impl Cinder {
    pub fn init(init_data: InitData) -> Result<Self, InitError> {
        let context = Context::init(init_data)?;
        Ok(Self { context })
    }

    pub fn set_view_color_clear(&mut self, id: ViewId, clear_op: ColorClear) {
        self.context.set_view_clear(id, Clear::Color(clear_op));
    }

    pub fn set_view_depth_clear(&mut self, id: ViewId, clear_op: DepthClear) {
        self.context.set_view_clear(id, Clear::Depth(clear_op));
    }

    pub fn set_view_rect(&mut self, id: ViewId, x: u32, y: u32, width: u32, height: u32) {
        self.context.set_view_rect(id, x, y, width, height)
    }

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
