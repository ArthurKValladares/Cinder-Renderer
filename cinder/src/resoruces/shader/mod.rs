use anyhow::Result;
use ash::vk;
use std::{fs::File, path::Path};

pub enum ShaderStage {
    Vertex,
    Fragment,
}

impl From<ShaderStage> for vk::ShaderStageFlags {
    fn from(stage: ShaderStage) -> Self {
        match stage {
            ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
            ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
        }
    }
}

pub struct ShaderDescription {
    pub stage: ShaderStage,
    pub path: &'static Path,
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
}

impl Shader {
    pub(crate) fn create(device: &ash::Device, desc: ShaderDescription) -> Result<Self> {
        let mut spv_file = File::open(desc.path)?;
        let code = ash::util::read_spv(&mut spv_file)?;
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe { device.create_shader_module(&shader_info, None)? };
        Ok(Shader { module })
    }
}
