use crate::{renderer::Renderer, ui::Ui};
use anyhow::Result;
use camera::{Camera, Direction, PerspectiveData};
use cinder::cinder::DefaultUniformBufferObject;
use egui_integration::EguiIntegration;
use input::{
    keyboard::{KeyboardInput, KeyboardState, VirtualKeyCode},
    mouse::MouseState,
};
use math::size::Size2D;
use winit::{event::WindowEvent, event_loop::EventLoop};

pub struct RuntimeState {
    pub camera: Camera,
    pub ui: Ui,
    pub egui: EguiIntegration,
    pub keyboard_state: KeyboardState,
    pub mouse_state: MouseState, // TODO: Keymap, move scene stuff here?
}

impl RuntimeState {
    pub fn new(event_loop: &EventLoop<()>, renderer: &mut Renderer) -> Self {
        let camera = camera::Camera::from_data(PerspectiveData::default());
        let ui = Ui::new();
        let egui = EguiIntegration::new(
            event_loop,
            renderer.device(),
            renderer.swapchain(),
            renderer.pipeline_cache(),
            renderer.surface_format(),
            ui.visuals(),
            ui.ui_scale(),
        )
        .expect("Could not create event loop");
        let keyboard_state = KeyboardState::default();
        let mouse_state = MouseState::default();
        Self {
            camera,
            ui,
            egui,
            keyboard_state,
            mouse_state,
        }
    }

    pub fn resize(&mut self, renderer: &Renderer) -> Result<()> {
        self.egui.resize(renderer.device())?;
        Ok(())
    }

    pub fn poll_event(&mut self, window_event: &WindowEvent) {
        self.egui.on_event(window_event);
    }

    pub fn get_camera_matrices(&self, surface_size: Size2D<u32>) -> DefaultUniformBufferObject {
        self.camera
            .get_matrices(surface_size.width() as f32, surface_size.height() as f32)
    }

    pub fn update_keyboard_state(&mut self, keyboard_input: KeyboardInput) {
        self.keyboard_state.update(keyboard_input);
    }

    pub fn update_position(&mut self) {
        // TODO: Clean this up
        if self.keyboard_state.is_down(VirtualKeyCode::W) {
            self.camera.update_position(Direction::Front);
        }
        if self.keyboard_state.is_down(VirtualKeyCode::S) {
            self.camera.update_position(Direction::Back);
        }
        if self.keyboard_state.is_down(VirtualKeyCode::A) {
            self.camera.update_position(Direction::Left);
        }
        if self.keyboard_state.is_down(VirtualKeyCode::D) {
            self.camera.update_position(Direction::Right);
        }
        if self.keyboard_state.is_down(VirtualKeyCode::Space) {
            self.camera.update_position(Direction::Up);
        }
        if self.keyboard_state.is_down(VirtualKeyCode::LShift) {
            self.camera.update_position(Direction::Down);
        }
    }

    pub fn rotate_camera(&mut self) {
        let delta = self.mouse_state.delta;
        self.camera.rotate(delta.x, delta.y);
    }
}
