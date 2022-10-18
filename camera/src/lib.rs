use math::{
    mat::Mat4,
    vec::{Vec2, Vec3},
};

static ROTATION_DELTA: f32 = 10.0;
static MOVEMENT_DELTA: f32 = 0.005;
static WORLD_UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

#[rustfmt::skip]
fn new_orthographic_proj(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> Mat4 {
    let w_inv = 1.0 / (right - left);
    let h_inv = 1.0 / (bottom - top);
    let d_inv = 1.0 / (far - near);
    Mat4::from_data(
        2.0 * w_inv, 0.0, 0.0, 0.0,
        0.0, 2.0 * h_inv, 0.0, -(bottom + top) * h_inv,
        0.0, 0.0, d_inv, 0.0,
        -(left + right) * w_inv, -(top + bottom) * h_inv, d_inv * near, 1.0,
    )
}

#[rustfmt::skip]
fn new_infinite_perspective_proj(aspect_ratio: f32, y_fov: f32, z_near: f32) -> Mat4 {
    let f = 1.0 / (y_fov * 0.5).tan();
    Mat4::from_data(
        f / aspect_ratio, 0.0, 0.0,  0.0,
        0.0,              f,   0.0,  0.0,
        0.0,              0.0, -1.0, -z_near,
        0.0,              0.0, -1.0, 0.0,
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
pub struct OrtographicData {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for OrtographicData {
    fn default() -> Self {
        Self {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: 1.0,
            near: 0.0,
            far: 100.0,
        }
    }
}
#[derive(Debug)]
pub struct PerspectiveData {
    pub aspect_ratio: Option<f32>,
    pub y_fov: f32,
    pub z_near: f32,
    pub z_far: Option<f32>,
}

impl Default for PerspectiveData {
    fn default() -> Self {
        Self {
            aspect_ratio: None,
            y_fov: 20.0,
            z_near: 0.01,
            z_far: None,
        }
    }
}

#[derive(Debug)]
pub enum CameraType {
    Orthographic(OrtographicData),
    Perspective(PerspectiveData),
}

impl CameraType {
    pub fn ortographic(data: OrtographicData) -> Self {
        Self::Orthographic(data)
    }

    pub fn perspective(data: PerspectiveData) -> Self {
        Self::Perspective(data)
    }

    pub fn projection(&self, window_width: f32, window_height: f32) -> CameraProjection {
        match self {
            Self::Orthographic(data) => CameraProjection::new_orthographic(data),
            Self::Perspective(data) => {
                CameraProjection::new_perspective(data, window_width, window_height)
            }
        }
    }
}

#[derive(Debug)]
pub enum CameraProjection {
    Orthographic(Mat4),
    Perspective(Mat4),
}

impl CameraProjection {
    pub fn new_orthographic(data: &OrtographicData) -> Self {
        Self::Orthographic(new_orthographic_proj(
            data.left,
            data.right,
            data.bottom,
            data.top,
            data.near,
            data.far,
        ))
    }

    pub fn new_perspective(data: &PerspectiveData, window_width: f32, window_height: f32) -> Self {
        let aspect_ratio = data.aspect_ratio.unwrap_or(window_width / window_height);
        let mat = new_infinite_perspective_proj(aspect_ratio, data.y_fov, data.z_near);
        Self::Perspective(mat)
    }

    pub fn to_raw_matrix(self) -> Mat4 {
        match self {
            CameraProjection::Orthographic(mat) => mat,
            CameraProjection::Perspective(mat) => mat,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraMatrices {
    proj_view: Mat4,
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
    ty: CameraType,
    rotation_speed: f32,
    movement_speed: f32,
    // TODO: Stop using yaw and pitch later
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn from_type(ty: CameraType) -> Self {
        Self {
            pos: Vec3::new(2.0, 2.0, 2.0),
            front: Vec3::new(1.0, 0.0, 0.0),
            ty,
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

    pub fn update_rotation_speed(&mut self, update: UpdateSpeed) {
        match update {
            UpdateSpeed::Decrease => self.rotation_speed -= ROTATION_DELTA,
            UpdateSpeed::Increase => self.rotation_speed += ROTATION_DELTA,
        }
    }

    pub fn update_movement_speed(&mut self, update: UpdateSpeed) {
        match update {
            UpdateSpeed::Decrease => self.movement_speed -= MOVEMENT_DELTA,
            UpdateSpeed::Increase => self.movement_speed += MOVEMENT_DELTA,
        }
        self.movement_speed = self.movement_speed.max(MOVEMENT_DELTA);
    }

    pub fn update_position(&mut self, direction: Direction) {
        let flat_front = Vec3::new(self.front.x(), 0.0, self.front.z());
        let left = WORLD_UP.cross(&flat_front).normalized();
        match direction {
            Direction::Front => self.pos += flat_front * self.movement_speed,
            Direction::Back => self.pos -= flat_front * self.movement_speed,
            Direction::Left => self.pos += left * self.movement_speed,
            Direction::Right => self.pos -= left * self.movement_speed,
            Direction::Up => self.pos -= WORLD_UP * self.movement_speed,
            Direction::Down => self.pos += WORLD_UP * self.movement_speed,
        }
    }

    pub fn rotate(&mut self, delta: (f64, f64)) {
        let (x, y) = delta;
        self.yaw += x as f32 * self.rotation_speed;
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
        let proj = self
            .ty
            .projection(window_width, window_height)
            .to_raw_matrix();

        let proj_view = proj * view;
        CameraMatrices { proj_view }
    }
}
