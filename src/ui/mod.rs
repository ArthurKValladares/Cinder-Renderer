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
}

impl Ui {
    pub const fn new() -> Self {
        Self {
            tabs: [Tab::App, Tab::Egui],
            selected_tab: None,
        }
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
                    egui::Window::new(Tab::App.name()).show(context, |ui| {
                        app_callback(ui);
                    });
                }
                Tab::Egui => {}
            }
        }
    }
}
