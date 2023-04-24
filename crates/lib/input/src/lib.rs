use math::point::Point2D;
use sdl2::{event::Event, keyboard::Keycode};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct KeyState {
    pub steps: u32,
}

impl KeyState {
    pub fn step(&mut self) {
        self.steps += 1;
    }
}

#[derive(Default, Clone)]
pub struct KeyboardState {
    keys_down: HashMap<Keycode, KeyState>,
    just_released: HashSet<Keycode>,
}

impl KeyboardState {
    pub fn is_down(&self, keycode: Keycode) -> bool {
        self.keys_down.contains_key(&keycode)
    }

    pub fn was_just_pressed(&self, keycode: Keycode) -> bool {
        self.keys_down
            .get(&keycode)
            .map_or(false, |state| state.steps == 1)
    }

    pub fn was_just_release(&self, keycode: Keycode) -> bool {
        self.just_released.contains(&keycode)
    }

    pub fn on_event(&mut self, event: &Event) {
        self.just_released.clear();
        match event {
            Event::KeyDown {
                keycode: Some(keycode),
                ..
            } => {
                let key_state = self
                    .keys_down
                    .entry(*keycode)
                    .or_insert(KeyState { steps: 0 });
                key_state.step();
            }
            Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => {
                self.keys_down.remove(keycode);
                self.just_released.insert(*keycode);
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct MouseState {
    position: Point2D<i32>,
    delta: Point2D<i32>,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            position: Point2D::zero(),
            delta: Point2D::zero(),
        }
    }
}

impl MouseState {
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position = Point2D::new(x, y);
    }

    pub fn position(&self) -> Point2D<i32> {
        self.position
    }

    pub fn delta(&self) -> Point2D<i32> {
        self.delta
    }

    pub fn reset_delta(&mut self) {
        self.delta = Point2D::new(0, 0);
    }

    pub fn on_event(&mut self, event: &Event) {
        match event {
            Event::MouseMotion { x, y, .. } => {
                self.delta = Point2D::new(x - self.position.x(), y - self.position.y());
                self.position = Point2D::new(*x, *y);
            }
            _ => {}
        }
    }
}
