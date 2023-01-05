#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct CubeConstants {
    pub transform: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CubeVertex {
    pub i_pos: [f32; 3],
    pub i_normal: [f32; 3],
}
