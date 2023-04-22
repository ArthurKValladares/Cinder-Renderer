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
