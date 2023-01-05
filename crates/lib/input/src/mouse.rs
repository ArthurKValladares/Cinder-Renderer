use math::vec::Vec2;
pub use winit::event::MouseButton;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, WindowEvent},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct MouseState {
    pub position: PhysicalPosition<f64>,
    pub delta: Vec2,
    pub buttons_held: u32,
    pub buttons_pressed: u32,
    pub buttons_released: u32,
}

fn button_id(mouse_button: &MouseButton) -> u32 {
    match mouse_button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        _ => 0,
    }
}

impl MouseState {
    pub fn update(&mut self, event: &Event<'_, ()>) {
        self.delta = Vec2::zero();

        self.buttons_released = 0;
        self.buttons_pressed = 0;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CursorMoved { position, .. } => {
                    self.position = *position;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let button_id = button_id(button);

                    match state {
                        ElementState::Pressed => {
                            self.buttons_held |= 1 << button_id;
                            self.buttons_pressed |= 1 << button_id;
                        }
                        ElementState::Released => {
                            self.buttons_held &= !(1 << button_id);
                            self.buttons_released |= 1 << button_id;
                        }
                    }
                }
                _ => (),
            },
            Event::DeviceEvent {
                device_id: _,
                event: winit::event::DeviceEvent::MouseMotion { delta },
            } => {
                self.delta.x += delta.0 as f32;
                self.delta.y += delta.1 as f32;
            }
            _ => (),
        }
    }

    pub fn was_just_pressed(&self, mouse_button: MouseButton) -> bool {
        (self.buttons_pressed & 1 << button_id(&mouse_button)) != 0
    }

    pub fn was_just_released(&self, mouse_button: MouseButton) -> bool {
        (self.buttons_released & 1 << button_id(&mouse_button)) != 0
    }

    pub fn is_held(&self, mouse_button: &MouseButton) -> bool {
        (self.buttons_held & 1 << button_id(&mouse_button)) != 0
    }
}
