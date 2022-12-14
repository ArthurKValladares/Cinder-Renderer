use std::fmt::Debug;

use egui_integration::egui;
use math::size::Size2D;
use serde::{Deserialize, Serialize};

// TODO: Tab abstraction
const EGUI_FILE: &str = "egui.json";
const TABS: [Tab; 2] = [Tab::App, Tab::Egui];
const CINDER_TABS: [CinderUiTab; 1] = [CinderUiTab::DepthBuffer];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
enum Tab {
    App,
    Egui,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Egui => "egui",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum CinderUiTab {
    DepthBuffer,
}

impl CinderUiTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DepthBuffer => "Depth Buffer",
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CinderUi {
    open: bool,
    selected_tab: Option<CinderUiTab>,
    fullscreen: bool,
}

impl CinderUi {
    fn render_depth_buffer_window(
        &mut self,
        context: &egui::Context,
        render_target_size: Size2D<u32>,
    ) {
        if !self.fullscreen {
            egui::Window::new("Depth Buffer")
                .open(&mut self.open)
                .show(context, |ui| {
                    let image_size = egui::Vec2::new(
                        render_target_size.width() as f32 / 4.0,
                        render_target_size.height() as f32 / 4.0,
                    );
                    let (_rect, _response) =
                        ui.allocate_exact_size(image_size, egui::Sense::drag());
                });
        } else {
            egui::Window::new("test")
                .open(&mut self.open)
                .show(context, |_ui| {});
        }
    }

    pub fn render_gui(&mut self, context: &egui::Context, render_target_size: Size2D<u32>) {
        egui::SidePanel::left("cinder")
            .resizable(false)
            .show_animated(context, self.open, |ui| {
                for tab in CINDER_TABS.iter() {
                    if ui
                        .selectable_label(
                            self.selected_tab
                                .map_or(false, |selected_tab| selected_tab == *tab),
                            tab.name(),
                        )
                        .clicked()
                    {
                        // TODO: Fix with if-let chains
                        if let Some(selected_tab) = self.selected_tab {
                            if selected_tab == *tab {
                                self.selected_tab = None;
                            } else {
                                self.selected_tab = Some(*tab);
                            }
                        } else {
                            self.selected_tab = Some(*tab);
                        }
                    }
                }

                if let Some(tab) = self.selected_tab {
                    match tab {
                        CinderUiTab::DepthBuffer => {
                            self.render_depth_buffer_window(context, render_target_size);
                        }
                    }
                }
            });
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Ui {
    cinder_ui: CinderUi,
    selected_tab: Option<Tab>,
    ui_data: UiData,
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
            if ui.selectable_label(self.cinder_ui.open, "cinder").clicked() {
                self.cinder_ui.open = !self.cinder_ui.open;
            }
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
                    // TODO: Fix with if-let chains
                    if let Some(selected_tab) = self.selected_tab {
                        if selected_tab == *tab {
                            self.selected_tab = None;
                        } else {
                            self.selected_tab = Some(*tab);
                        }
                    } else {
                        self.selected_tab = Some(*tab);
                    }
                }
            }
        });
    }

    pub fn show_selected_tab(
        &mut self,
        context: &egui::Context,
        render_target_size: Size2D<u32>,
        app_callback: impl FnOnce(&mut egui::Ui),
    ) {
        self.cinder_ui.render_gui(context, render_target_size);

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

    pub fn depth_image_open(&self) -> bool {
        self.cinder_ui.open && self.cinder_ui.selected_tab == Some(CinderUiTab::DepthBuffer)
    }

    pub fn flip_fullscreen(&mut self) {
        self.cinder_ui.fullscreen = !self.cinder_ui.fullscreen;
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Ok(as_string) = serde_json::to_string_pretty(self) {
            std::fs::write(EGUI_FILE, as_string).ok();
        }
    }
}
