use anyhow::Result;
use cinder::device::Device;
use winit::{event_loop::EventLoopWindowTarget, window::Window};

pub struct EguiIntegration {
    egui_context: egui::Context,
    egui_winit: egui_winit::State,
}

impl EguiIntegration {
    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>, device: &Device) -> Result<Self> {
        let egui_context = egui::Context::default();
        egui_context.set_visuals(egui::Visuals::light());
        let egui_winit = egui_winit::State::new(event_loop);

        Ok(Self {
            egui_context,
            egui_winit,
        })
    }
}
