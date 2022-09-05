use crate::{
    backend::{back, back::RendererContext, AsRendererContext},
    init::{Init, InitData},
    view::{Clear, View, ViewId, MAX_VIEWS},
};
use math::{point::Point2D, rect::Rect2D, size::Size2D};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ContextError {
    // TODO: Need to figure out how to make this work across backends
    #[error("Could not create renderer context")]
    RendererInitError,
}

pub struct Context {
    init: Init,
    views: [View; MAX_VIEWS],
    renderer_context: RendererContext,
}

impl Context {
    pub fn init(window: &winit::window::Window, data: InitData) -> Result<Self, ContextError> {
        let views = [View::from_resolution(data.backbuffer_resolution); MAX_VIEWS];
        let init = Init::from_data(&data);
        let renderer_context = <RendererContext as AsRendererContext>::create(window, data)
            .map_err(|_| ContextError::RendererInitError)?;
        Ok(Self {
            init,
            views,
            renderer_context,
        })
    }

    pub fn set_view_clear(&mut self, id: ViewId, clear_op: Clear) {
        self.views[id.idx() as usize].clear = clear_op;
    }

    pub fn set_view_rect(&mut self, id: ViewId, x: u32, y: u32, width: u32, height: u32) {
        self.views[id.idx() as usize].rect = Rect2D::from_top_left(
            Point2D::new(x as f32, y as f32),
            Size2D::new(width as f32, height as f32),
        );
    }
}
