use egui::Context;

use crate::{EguiIntegration, DEFAULT_PPP};

pub struct SharedEguiMenu {
    pixels_per_point: f32,
    should_set_ppp: bool,
}

impl Default for SharedEguiMenu {
    fn default() -> Self {
        Self {
            pixels_per_point: DEFAULT_PPP,
            should_set_ppp: false,
        }
    }
}

impl SharedEguiMenu {
    pub fn draw(&mut self, context: &Context) {
        egui::Window::new("Shared Menu").show(context, |ui| {
            let ret =
                ui.add(egui::Slider::new(&mut self.pixels_per_point, 1.0..=4.0).text("UI Scale"));
            if ret.drag_released() {
                self.should_set_ppp = true;
            }
        });
    }

    pub fn update(&mut self, integration: &mut EguiIntegration) {
        if self.should_set_ppp {
            integration.set_pixels_per_point(self.pixels_per_point);
            self.should_set_ppp = false;
        }
    }
}
