#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct DepthUniformBufferObject {
    pub model: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct DepthVertex {
    pub i_pos: [f32; 3],
    pub i_normal: [f32; 3],
}
