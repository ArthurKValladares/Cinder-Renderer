#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct HotReloadVertex {
    pub i_pos: [f32; 2],
    pub i_uv: [f32; 2],
}
