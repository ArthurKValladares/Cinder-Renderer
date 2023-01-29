#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct DepthTextureVertex {
    pub i_pos: [f32; 2],
    pub i_uv: [f32; 2],
}
