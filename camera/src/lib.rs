use math::vec::Vec3;

static WORLD_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Front,
    Back,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug)]
pub struct Camera {
    pub movement_speed: f32,
    pub pos: Vec3,
    pub front: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            movement_speed: 0.01,
            pos: Vec3::new(2.0, 2.0, 2.0),
            front: Vec3::new(1.0, 0.0, 0.0),
        }
    }
}

impl Camera {
    pub fn update_position(&mut self, direction: Direction) {
        let flat_front = Vec3::new(self.front.x(), 0.0, self.front.z());
        let left = WORLD_UP.cross(&flat_front).normalized();
        match direction {
            Direction::Front => self.pos += flat_front * self.movement_speed,
            Direction::Back => self.pos -= flat_front * self.movement_speed,
            Direction::Left => self.pos += left * self.movement_speed,
            Direction::Right => self.pos -= left * self.movement_speed,
            Direction::Up => self.pos += WORLD_UP * self.movement_speed,
            Direction::Down => self.pos -= WORLD_UP * self.movement_speed,
        }
    }
}
