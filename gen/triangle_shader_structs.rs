#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct TriangleVertex {
    pub i_pos: [f32; 2],
    pub i_color: [f32; 4],
}
