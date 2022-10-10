use egui_integration::egui;

pub struct UiTab {
    pub name: &'static str,
    // TODO: More stuff
}

impl UiTab {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

const APP: UiTab = UiTab::new("App");
const EGUI: UiTab = UiTab::new("Egui");

pub struct Ui {
    tabs: [UiTab; 2],
    selected_tab: usize,
}

impl Ui {
    pub const fn new() -> Self {
        Self {
            tabs: [APP, EGUI],
            selected_tab: 0,
        }
    }

    pub fn show_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab_idx, tab) in self.tabs.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_tab == tab_idx, tab.name)
                    .clicked()
                {
                    self.selected_tab = tab_idx;
                }
            }
        });
    }
}
