use input::{KeyboardState, MouseState};
use math::{mat::Mat4, quat::Quat, vec::Vec3};
use sdl2::keyboard::Keycode;

pub use input;

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
    pub y_fov: f32,
    pub z_near: f32,
    pub world_up: Vec3,
    pub movement_per_sec: f32,
    // Amount we will rotate the camera by if we move the curse of screens's woth in the respective dimension
    pub rotation_speed: f32,
    pub flipped_y: bool,
}

impl Default for CameraDescription {
    fn default() -> Self {
        Self {
            y_fov: 30.0,
            z_near: 0.01,
            world_up: Vec3::new(0.0, 1.0, 0.0),
            movement_per_sec: 1.0,
            rotation_speed: 180.0_f32.to_radians(),
            flipped_y: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    position: Vec3,
    front: Vec3,
    world_up: Vec3,
    yaw: f32,
    pitch: f32,
    y_fov: f32,
    z_near: f32,
    movement_per_sec: f32,
    rotation_speed: f32,
    flipped_y: bool,
}

impl Camera {
    pub fn new(position: Vec3, front: Vec3, desc: CameraDescription) -> Self {
        debug_assert!(
            front.is_normal(),
            "Front vector passed to camera must be normalized"
        );
        debug_assert!(
            desc.world_up.is_normal(),
            "World Up vector passed to camera must be normalized"
        );
        Self {
            position,
            front: front.normalized(),
            world_up: desc.world_up,
            yaw: 0.0,
            pitch: 0.0,
            y_fov: desc.y_fov,
            z_near: desc.z_near,
            movement_per_sec: desc.movement_per_sec,
            rotation_speed: desc.rotation_speed,
            flipped_y: desc.flipped_y,
        }
    }

    pub fn projection(&self, surface_width: f32, surface_height: f32) -> Mat4 {
        new_infinite_perspective_proj(surface_width / surface_height, self.y_fov, self.z_near)
    }

    pub fn view(&self) -> Mat4 {
        look_to(self.position, self.front, self.world_up)
    }

    pub fn update(
        &mut self,
        keyboard_state: &KeyboardState,
        mouse_state: &MouseState,
        screen_width: u32,
        screen_height: u32,
        last_dt: Option<u128>,
    ) {
        if let Some(dt) = last_dt {
            let mouse_delta = mouse_state.delta();

            self.yaw += mouse_delta.x() as f32 / screen_width as f32 * self.rotation_speed;
            let scaled_y_delta =
                mouse_delta.y() as f32 / screen_height as f32 * self.rotation_speed;
            self.pitch += if self.flipped_y {
                scaled_y_delta
            } else {
                -scaled_y_delta
            };
            self.pitch = self.pitch.clamp(-89.9, 89.9);
            self.front = Vec3::new(
                self.yaw.cos() * self.pitch.cos(),
                self.pitch.sin(),
                self.yaw.sin() * self.pitch.cos(),
            );

            let right = self.front.cross(&self.world_up).normalized();
            let up = self.front.cross(&right).normalized();

            let disp = {
                let mut disp = Vec3::zero();

                if keyboard_state.is_down(Keycode::W) {
                    disp += self.front;
                }
                if keyboard_state.is_down(Keycode::S) {
                    disp -= self.front;
                }

                if keyboard_state.is_down(Keycode::D) {
                    disp += right;
                }
                if keyboard_state.is_down(Keycode::A) {
                    disp -= right;
                }

                if keyboard_state.is_down(Keycode::Space) {
                    disp += up;
                }
                if keyboard_state.is_down(Keycode::LShift) {
                    disp -= up;
                }

                let dt_scale = dt as f32 / 1000.0;

                if disp == Vec3::zero() {
                    disp
                } else {
                    disp.normalized() * dt_scale * self.movement_per_sec
                }
            };
            self.position += disp;
        }
    }
}
