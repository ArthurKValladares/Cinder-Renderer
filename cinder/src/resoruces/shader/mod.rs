use std::path::Path;

use ash::vk;

pub struct ShaderDescription {
    pub path: &'static Path,
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
}
