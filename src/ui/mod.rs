use egui_integration::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

pub struct Ui {
    tabs: [Tab; 2],
    selected_tab: Option<Tab>,
    visuals: egui::Visuals,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            tabs: [Tab::App, Tab::Egui],
            selected_tab: None,
            visuals: egui::Visuals::light(),
        }
    }

    pub fn visuals(&self) -> egui::Visuals {
        self.visuals.clone()
    }

    pub fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for tab in self.tabs.iter() {
                if ui
                    .selectable_label(
                        self.selected_tab
                            .map_or_else(|| false, |selected_tab| selected_tab == *tab),
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
        if let Some(selected_tab) = self.selected_tab {
            match selected_tab {
                Tab::App => {
                    // TODO: window type configurable
                    egui::Window::new(Tab::App.name())
                        .resizable(true)
                        .show(context, |ui| {
                            app_callback(ui);
                        });
                }
                Tab::Egui => {
                    egui::Window::new(Tab::Egui.name())
                        .resizable(true)
                        .show(context, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Style:");
                                if ui
                                    .selectable_label(self.visuals.dark_mode, "dark")
                                    .clicked()
                                {
                                    self.visuals = egui::Visuals::dark();
                                    context.set_visuals(self.visuals());
                                }
                                if ui
                                    .selectable_label(!self.visuals.dark_mode, "light")
                                    .clicked()
                                {
                                    self.visuals = egui::Visuals::light();
                                    context.set_visuals(self.visuals());
                                }
                            })
                        });
                }
            }
        }
    }
}
