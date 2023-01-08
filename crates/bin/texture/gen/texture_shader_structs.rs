#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct TextureVertex {
    pub i_pos: [f32; 2],
    pub i_uv: [f32; 2],
}
