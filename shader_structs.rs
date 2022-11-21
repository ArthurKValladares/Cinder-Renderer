#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DefaultUniformBufferObject {
    pub proj: [f32; 4],
    pub view: [f32; 4],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[derive(rkvy::Archive, rkvy::Deserialize, rkvy::Serialize)]
pub struct DefaultVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub pad: [f32; 2],
}
