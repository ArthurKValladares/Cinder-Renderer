use std::path::Path;

use ash::vk;

pub enum ShaderStage {
    Vertex,
    Fragment,
}

pub struct ShaderDescription {
    pub stage: ShaderStage,
    pub path: &'static Path,
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
}
