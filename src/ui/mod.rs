use egui_integration::egui;
use serde::{Deserialize, Serialize};

const EGUI_FILE: &str = "egui.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
enum Tab {
    App,
    Egui,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::App => "app",
            Tab::Egui => "egui",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Ui {
    tabs: [Tab; 2],
    selected_tab: Tab,
    dark_mode: bool,
    ui_scale: f32,
    open: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            tabs: [Tab::App, Tab::Egui],
            selected_tab: Tab::App,
            dark_mode: false,
            ui_scale: 1.0,
            open: true,
        }
    }
}

impl Ui {
    pub fn new() -> Self {
        if let Ok(buf) = std::fs::read(EGUI_FILE) {
            serde_json::from_slice(&buf).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    pub fn visuals(&self) -> egui::Visuals {
        match self.dark_mode {
            true => egui::Visuals::dark(),
            false => egui::Visuals::light(),
        }
    }

    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    pub fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for tab in self.tabs.iter() {
                if ui
                    .selectable_label(self.selected_tab == *tab, tab.name())
                    .clicked()
                {
                    self.selected_tab = *tab;
                    self.open = true;
                }
            }
        });
    }

    pub fn show_selected_tab(
        &mut self,
        context: &egui::Context,
        app_callback: impl FnOnce(&mut egui::Ui),
    ) {
        let mut open = self.open;
        match self.selected_tab {
            Tab::App => {
                egui::Window::new(Tab::App.name())
                    .open(&mut open)
                    .show(context, |ui| {
                        app_callback(ui);
                    });
            }
            Tab::Egui => {
                egui::Window::new(Tab::Egui.name())
                    .open(&mut open)
                    .show(context, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Style:");
                            if ui.selectable_label(self.dark_mode, "dark").clicked() {
                                self.dark_mode = true;
                                context.set_visuals(self.visuals());
                            }
                            if ui.selectable_label(!self.dark_mode, "light").clicked() {
                                self.dark_mode = false;
                                context.set_visuals(self.visuals());
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("UI Scale:");
                            let res = ui.add(egui::Slider::new(&mut self.ui_scale, 0.5..=3.0));
                            if res.drag_released() {
                                context.set_pixels_per_point(self.ui_scale);
                            }
                        })
                    });
            }
        }
        self.open = open;
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Ok(as_string) = serde_json::to_string_pretty(self) {
            std::fs::write(EGUI_FILE, as_string).ok();
        }
    }
}
