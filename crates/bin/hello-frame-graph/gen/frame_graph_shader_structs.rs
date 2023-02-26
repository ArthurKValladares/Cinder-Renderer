#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct FrameGraphUniformBufferObject {
    pub model: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct FrameGraphVertex {
    pub i_pos: [f32; 3],
    pub i_uv: [f32; 2],
}
