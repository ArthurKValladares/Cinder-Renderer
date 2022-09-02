use crate::init::Resolution;
use math::{point::Point2D, rect::Rect2D, size::Size2D};

// TODO: can find a better value for this later
pub const MAX_VIEWS: usize = 64;

#[derive(Debug, Clone, Copy)]
pub enum ColorClear {
    None,
    Value([u8; 4]),
}

#[derive(Debug, Clone, Copy)]
pub enum DepthClear {
    None,
    Value(f32),
}

#[derive(Debug, Clone, Copy)]
pub enum Clear {
    Color(ColorClear),
    Depth(DepthClear),
}

#[derive(Debug, Clone, Copy)]
pub struct ViewId(pub u32);

impl ViewId {
    pub(crate) fn idx(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct View {
    pub(crate) clear: Clear,
    pub(crate) rect: Rect2D,
}

impl View {
    pub fn from_resolution(resolution: Resolution) -> Self {
        Self {
            clear: Clear::Color(ColorClear::Value([0, 0, 0, 0])),
            rect: Rect2D::from_top_left(
                Point2D::new(0.0, 0.0),
                Size2D::new(resolution.width as f32, resolution.height as f32),
            ),
        }
    }
}
