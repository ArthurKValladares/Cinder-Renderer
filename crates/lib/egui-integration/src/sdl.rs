use sdl2::{event::Event, mouse::MouseButton};

fn translate_mouse_button(button: &MouseButton) -> Option<egui::PointerButton> {
    match button {
        MouseButton::Left => Some(egui::PointerButton::Primary),
        MouseButton::Right => Some(egui::PointerButton::Secondary),
        MouseButton::Middle => Some(egui::PointerButton::Middle),
        MouseButton::X1 => Some(egui::PointerButton::Extra1),
        MouseButton::X2 => Some(egui::PointerButton::Extra2),
        _ => None,
    }
}

#[must_use]
pub struct EventResponse {
    pub consumed: bool,
}

#[derive(Debug)]
pub struct EguiSdl {
    egui_input: egui::RawInput,
    current_pixels_per_point: f32,
}

impl EguiSdl {
    pub fn new() -> Self {
        Self {
            egui_input: Default::default(),
            current_pixels_per_point: 1.0,
        }
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.current_pixels_per_point
    }

    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
        self.egui_input.pixels_per_point = Some(pixels_per_point);
        self.current_pixels_per_point = pixels_per_point;
    }

    pub fn on_event(&mut self, egui_ctx: &egui::Context, event: &Event) -> EventResponse {
        match event {
            Event::MouseMotion { x, y, .. } => {
                self.on_mouse_motion(x, y);
                EventResponse {
                    consumed: egui_ctx.is_using_pointer(),
                }
            }
            Event::MouseButtonDown {
                mouse_btn, x, y, ..
            } => {
                self.on_mouse_down(mouse_btn, x, y);
                EventResponse {
                    consumed: egui_ctx.wants_pointer_input(),
                }
            }
            Event::MouseButtonUp {
                mouse_btn, x, y, ..
            } => {
                self.on_mouse_up(mouse_btn, x, y);
                EventResponse {
                    consumed: egui_ctx.wants_pointer_input(),
                }
            }
            _ => EventResponse { consumed: false },
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let screen_size_in_pixels = egui::vec2(width as f32, height as f32);
        let screen_size_in_points = screen_size_in_pixels / self.pixels_per_point();
        self.egui_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            screen_size_in_points,
        ));
    }

    pub fn take_egui_input(&mut self) -> egui::RawInput {
        self.egui_input.take()
    }

    pub fn handle_platform_output(
        &mut self,
        egui_ctx: &egui::Context,
        platform_output: egui::PlatformOutput,
    ) {
        let egui::PlatformOutput { .. } = platform_output;
        self.current_pixels_per_point = egui_ctx.pixels_per_point();
    }

    fn normalize_pos(&self, x: i32, y: i32) -> egui::Pos2 {
        egui::pos2(
            x as f32 / self.pixels_per_point(),
            y as f32 / self.pixels_per_point(),
        )
    }

    fn on_mouse_down(&mut self, mouse_btn: &MouseButton, x: &i32, y: &i32) {
        if let Some(button) = translate_mouse_button(mouse_btn) {
            let pos = self.normalize_pos(*x, *y);

            self.egui_input.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: self.egui_input.modifiers,
            });
        }
    }

    fn on_mouse_up(&mut self, mouse_btn: &MouseButton, x: &i32, y: &i32) {
        if let Some(button) = translate_mouse_button(mouse_btn) {
            let pos = self.normalize_pos(*x, *y);

            self.egui_input.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: self.egui_input.modifiers,
            });
        }
    }

    fn on_mouse_motion(&mut self, x: &i32, y: &i32) {
        let pos = self.normalize_pos(*x, *y);

        self.egui_input.events.push(egui::Event::PointerMoved(pos));
    }
}
