use ash::vk;
use std::{fs::File, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderCreateError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    VulkanError(#[from] ash::vk::Result),
}

pub struct Shader {
    shader_module: vk::ShaderModule,
    // TODO: Need to get this from reflection on the shader later
    //binding: vk::DescriptorSetLayoutBinding,
}

impl Shader {
    pub fn create(
        device: &ash::Device,
        spv_path: impl AsRef<Path>,
    ) -> Result<Self, ShaderCreateError> {
        let spv_path = spv_path.as_ref();
        let mut spv_file = File::open(spv_path)?;
        let code = ash::util::read_spv(&mut spv_file)?;

        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);

        let shader_module = unsafe { device.create_shader_module(&shader_info, None)? };
        Ok(Self { shader_module })
    }
}
