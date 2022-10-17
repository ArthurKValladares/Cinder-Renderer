use math::{size::Size2D, vec::Vec2};

#[derive(Clone, Copy, Debug, Default)]
pub struct MouseState {
    pub delta: Option<Vec2>,
}

impl MouseState {
    pub fn update(&mut self, window_size: Size2D<u32>, cursor_delta: Option<(f32, f32)>) {
        let checked_div = |numerator, denominator| {
            if denominator == 0 {
                0.0 as f32
            } else {
                numerator as f32 / denominator as f32
            }
        };

        self.delta = cursor_delta.map(|(x, y)| {
            Vec2::new(
                checked_div(x, window_size.width()),
                checked_div(y, window_size.height()),
            )
        });
    }
}
