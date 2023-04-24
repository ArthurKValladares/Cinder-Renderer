use anyhow::Result;
use sdl2::{mouse::MouseState, video::Window, EventPump, Sdl};

#[derive(Debug)]
pub struct WindowDescription<'a> {
    pub title: &'a str,
    pub relative_mouse: bool,
}

impl<'a> Default for WindowDescription<'a> {
    fn default() -> Self {
        Self {
            title: "sdl-window",
            relative_mouse: false,
        }
    }
}

pub struct SdlContext {
    pub sdl: Sdl,
    pub event_pump: EventPump,
    pub window: Window,
}

impl SdlContext {
    pub fn new(width: u32, height: u32, window_description: WindowDescription) -> Result<Self> {
        let sdl = sdl2::init().unwrap();
        let event_pump = sdl.event_pump().unwrap();
        let window = {
            let mut window_builder =
                sdl.video()
                    .unwrap()
                    .window(window_description.title, width, height);
            window_builder.position_centered().resizable();
            if cfg!(target_os = "macos") {
                window_builder.metal_view();
            }
            window_builder.build().unwrap()
        };

        sdl.mouse()
            .warp_mouse_in_window(&window, width as i32 / 2, height as i32 / 2);
        sdl.mouse()
            .set_relative_mouse_mode(window_description.relative_mouse);

        Ok(Self {
            sdl,
            event_pump,
            window,
        })
    }
}
