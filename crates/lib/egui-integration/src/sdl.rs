use sdl2::{event::Event, video::Window};

#[must_use]
pub struct EventResponse {
    pub consumed: bool,
}

pub struct EguiSdl {}

impl EguiSdl {
    pub fn new(window: &Window) -> Self {
        Self {}
    }

    pub fn on_event(&mut self, event: &Event) -> EventResponse {
        match event {
            mouse_motion @ Event::MouseMotion { .. } => {
                println!("MouseMotion {mouse_motion:#?}");
                EventResponse { consumed: false }
            }
            mouse_button_down @ Event::MouseButtonDown { .. } => {
                println!("MouseButtonDown {mouse_button_down:#?}");
                EventResponse { consumed: false }
            }
            mouse_button_up @ Event::MouseButtonUp { .. } => {
                println!("MouseButtonUp {mouse_button_up:#?}");
                EventResponse { consumed: false }
            }
            _ => EventResponse { consumed: false },
        }
    }
}
