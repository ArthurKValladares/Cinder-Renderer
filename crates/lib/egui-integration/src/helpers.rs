use egui::{Response, Ui};

use crate::EguiIntegration;

pub struct HelperEguiMenu {
    pixels_per_point: f32,
    // TODO: Just trying stuff out
    should_set_ppp: bool,
}

impl Default for HelperEguiMenu {
    fn default() -> Self {
        Self {
            pixels_per_point: 3.0,
            should_set_ppp: false,
        }
    }
}

impl HelperEguiMenu {
    pub fn draw(&mut self, ui: &mut Ui) {
        let ret = ui.add(egui::Slider::new(&mut self.pixels_per_point, 1.0..=4.0).text("UI Scale"));
        if ret.drag_released() {
            self.should_set_ppp = true;
        }
    }

    pub fn update(&mut self, integration: &mut EguiIntegration) {
        if self.should_set_ppp {
            integration.set_pixels_per_point(self.pixels_per_point);
            self.should_set_ppp = false;
        }
    }
}
