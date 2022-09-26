use anyhow::Result;
use cinder::{context::render_context::RenderContext, device::Device};
use egui::{RawInput, TexturesDelta};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

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

    pub fn on_event(&mut self, event: &WindowEvent<'_>) {
        self.egui_winit.on_event(&self.egui_context, event);
    }

    pub fn run(
        &mut self,
        device: &Device,
        context: &RenderContext,
        present_index: u32,
        window: &Window,
        f: impl FnOnce(&egui::Context),
    ) {
        let raw_input = self.gather_input(window);
        // TODO: Hook up needs_repaint
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            repaint_after,
        } = self.egui_context.run(raw_input, f);

        let clipped_primitives = self.egui_context.tessellate(shapes);

        self.egui_winit
            .handle_platform_output(window, &self.egui_context, platform_output);

        // TODO? Make this a separate step
        self.set_textures(device, context, &textures_delta);
        // TODO: render
        self.free_textures(textures_delta);
    }

    pub fn resize(&mut self, device: &Device) {}

    pub fn clean(&mut self, device: &Device) {}

    fn gather_input(&mut self, window: &Window) -> RawInput {
        self.egui_winit.take_egui_input(window)
    }

    fn set_textures(
        &mut self,
        device: &Device,
        context: &RenderContext,
        textures_delta: &TexturesDelta,
    ) {
    }

    fn free_textures(&mut self, _textures_delta: TexturesDelta) {}
}
