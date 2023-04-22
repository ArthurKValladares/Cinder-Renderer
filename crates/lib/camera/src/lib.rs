use math::{mat::Mat4, point::Point3D, vec::Vec3};

#[rustfmt::skip]
pub fn new_infinite_perspective_proj(aspect_ratio: f32, y_fov: f32, z_near: f32) -> Mat4 {
    let f = 1.0 / (y_fov / 2.0).tan();
    Mat4::from_data(
        f / aspect_ratio, 0., 0.0, 0.0,
        0.0,              f,  0.0, 0.0,
        0.0,              0., 0.0, z_near,
        0.0,              0., 1.0, 0.0,
    )
}

#[rustfmt::skip]
pub fn look_to(eye: Vec3, front: Vec3, world_up: Vec3) -> Mat4 {
    let front = front.normalized();
    let side = world_up.cross(&front).normalized();
    let up = front.cross(&side);

    Mat4::from_data(
        side.x(),  side.y(),  side.z(),  -side.dot(&eye),
        up.x(),    up.y(),    up.z(),    -up.dot(&eye),
        front.x(), front.y(), front.z(), -front.dot(&eye),
        0.0,       0.0,       0.0,       1.0,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct CameraDescription {
    y_fov: f32,
    z_near: f32,
    world_up: Vec3,
}

impl Default for CameraDescription {
    fn default() -> Self {
        Self {
            y_fov: 30.0,
            z_near: 0.01,
            world_up: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    position: Vec3,
    front: Vec3,
    y_fov: f32,
    z_near: f32,
}

impl Camera {
    pub fn new(position: Vec3, front: Vec3, desc: CameraDescription) -> Self {
        Self {
            position,
            front,
            y_fov: desc.y_fov,
            z_near: desc.z_near,
        }
    }

    pub fn projection(&self, surface_width: f32, surface_height: f32) -> Mat4 {
        new_infinite_perspective_proj(surface_width / surface_height, self.y_fov, self.z_near)
    }

    pub fn view(&self) -> Mat4 {
        look_to(self.position, self.front, Vec3::new(0.0, 1.0, 0.0))
    }
}
