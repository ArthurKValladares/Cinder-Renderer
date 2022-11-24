#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct DefaultUniformBufferObject {
    pub proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct DefaultVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub pad: [f32; 2],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Defaultconstants {
    pub pc_color: [f32; 4],
}
