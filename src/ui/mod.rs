use egui_integration::egui;
use serde::{Deserialize, Serialize};

const EGUI_FILE: &str = "egui.json";
const TABS: [Tab; 2] = [Tab::App, Tab::Egui];

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
struct UiData {
    dark_mode: bool,
    ui_scale: f32,
}

impl Default for UiData {
    fn default() -> Self {
        Self {
            dark_mode: false,
            ui_scale: 1.0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Ui {
    selected_tab: Option<Tab>,
    ui_data: UiData,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            selected_tab: None,
            ui_data: Default::default(),
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
        match self.ui_data.dark_mode {
            true => egui::Visuals::dark(),
            false => egui::Visuals::light(),
        }
    }

    pub fn ui_scale(&self) -> f32 {
        self.ui_data.ui_scale
    }

    pub fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.separator();
            for tab in TABS.iter() {
                if ui
                    .selectable_label(
                        self.selected_tab
                            .map_or(false, |selected_tab| selected_tab == *tab),
                        tab.name(),
                    )
                    .clicked()
                {
                    self.selected_tab = Some(*tab);
                }
            }
        });
    }

    pub fn show_selected_tab(
        &mut self,
        context: &egui::Context,
        app_callback: impl FnOnce(&mut egui::Ui),
    ) {
        let mut open = self.selected_tab.is_some();
        if let Some(tab) = self.selected_tab {
            match tab {
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
                                if ui
                                    .selectable_label(self.ui_data.dark_mode, "dark")
                                    .clicked()
                                {
                                    self.ui_data.dark_mode = true;
                                    context.set_visuals(self.visuals());
                                }
                                if ui
                                    .selectable_label(!self.ui_data.dark_mode, "light")
                                    .clicked()
                                {
                                    self.ui_data.dark_mode = false;
                                    context.set_visuals(self.visuals());
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("UI Scale:");
                                let res = ui
                                    .add(egui::Slider::new(&mut self.ui_data.ui_scale, 0.5..=3.0));
                                if res.drag_released() {
                                    context.set_pixels_per_point(self.ui_data.ui_scale);
                                }
                            })
                        });
                }
            }
        }
        if !open {
            self.selected_tab = None;
        }
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Ok(as_string) = serde_json::to_string_pretty(self) {
            std::fs::write(EGUI_FILE, as_string).ok();
        }
    }
}
