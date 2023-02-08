#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct BindlessUniformBufferObject {
    pub model: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct BindlessVertex {
    pub pos: [f32; 4],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
