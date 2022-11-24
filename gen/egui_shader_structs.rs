#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Eguiconstants {
    pub screen_size: [f32; 2],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct EguiVertex {
    pub i_pos: [f32; 2],
    pub i_uv: [f32; 2],
    pub i_color_lowp: [u8; 4],
}
