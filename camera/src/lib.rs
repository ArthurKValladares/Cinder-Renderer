use math::{mat::Mat4, vec::Vec3};

pub static ROTATION_DELTA: f32 = 0.01;
pub static MOVEMENT_DELTA: f32 = 0.001;
static WORLD_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

fn new_infinite_perspective_proj(aspect_ratio: f32, y_fov: f32, z_near: f32) -> Mat4 {
    let f = 1.0 / (y_fov / 2.0).tan();
    Mat4::from_data(
        f / aspect_ratio,
        0.,
        0.,
        0.,
        //
        0.,
        f,
        0.,
        0.,
        //
        0.,
        0.,
        0.,
        z_near,
        //
        0.,
        0.,
        1.0,
        0.,
    )
}

#[rustfmt::skip]
fn look_to(eye: Vec3, front: Vec3, world_up: Vec3) -> Mat4 {
    let front = (front * -1.0).normalized();
    let side = world_up.cross(&front).normalized();
    let up = front.cross(&side);

    Mat4::from_data(
        side.x(),  side.y(),  side.z(),  -side.dot(&eye),
        up.x(),    up.y(),    up.z(),    -up.dot(&eye),
        front.x(), front.y(), front.z(), -front.dot(&eye),
        0.0,       0.0,       0.0,       1.0,
    )
}

#[derive(Debug)]
pub struct PerspectiveData {
    pub y_fov: f32,
    pub z_near: f32,
}

impl Default for PerspectiveData {
    fn default() -> Self {
        Self {
            y_fov: 20.0,
            z_near: 0.01,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraMatrices {
    proj: Mat4,
    view: Mat4,
}

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Front,
    Back,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Copy, Clone)]
pub enum UpdateSpeed {
    Decrease,
    Increase,
}

#[derive(Debug)]
pub struct Camera {
    pos: Vec3,
    front: Vec3,
    data: PerspectiveData,
    pub rotation_speed: f32,
    pub movement_speed: f32,
    // TODO: Stop using yaw and pitch later
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn from_data(data: PerspectiveData) -> Self {
        Self {
            pos: Vec3::new(2.0, 2.0, 2.0),
            front: Vec3::new(1.0, 0.0, 0.0),
            data,
            rotation_speed: 0.1,
            movement_speed: 0.01,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn pos(&self) -> &Vec3 {
        &self.pos
    }

    pub fn front(&self) -> &Vec3 {
        &self.front
    }

    pub fn update_position(&mut self, direction: Direction) {
        let flat_front = Vec3::new(self.front.x(), 0.0, self.front.z());
        let left = WORLD_UP.cross(&flat_front).normalized();
        match direction {
            Direction::Front => self.pos -= flat_front * self.movement_speed,
            Direction::Back => self.pos += flat_front * self.movement_speed,
            Direction::Left => self.pos += left * self.movement_speed,
            Direction::Right => self.pos -= left * self.movement_speed,
            Direction::Up => self.pos += WORLD_UP * self.movement_speed,
            Direction::Down => self.pos -= WORLD_UP * self.movement_speed,
        }
    }

    pub fn rotate(&mut self, delta: (f64, f64)) {
        let (x, y) = delta;
        self.yaw -= x as f32 * self.rotation_speed;
        self.pitch += y as f32 * self.rotation_speed;
        self.pitch = self.pitch.clamp(-89.0, 89.0);

        let yaw_r = self.yaw.to_radians();
        let pitch_r = self.pitch.to_radians();
        self.front = Vec3::new(
            yaw_r.cos() * pitch_r.cos(),
            pitch_r.sin(),
            yaw_r.sin() * pitch_r.cos(),
        );
    }

    pub fn get_matrices(&self, window_width: f32, window_height: f32) -> CameraMatrices {
        let eye = self.pos;
        let front = self.front;

        let view = look_to(eye, front, WORLD_UP);
        let proj = new_infinite_perspective_proj(
            window_width / window_height,
            self.data.y_fov,
            self.data.z_near,
        );

        CameraMatrices { proj, view }
    }
}
