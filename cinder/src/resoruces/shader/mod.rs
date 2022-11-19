use anyhow::Result;
use ash::vk;
use rust_shader_tools::ShaderData;
use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::Path,
};

#[derive(Debug, Clone, Copy)]
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
    pub bytes: &'static [u8],
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
    reflect_data: ShaderData,
}

impl Shader {
    pub(crate) fn create(device: &ash::Device, desc: ShaderDescription) -> Result<Self> {
        let reflect_data = ShaderData::from_spv(desc.bytes)?;
        let mut spv_file = Cursor::new(desc.bytes);
        let code = ash::util::read_spv(&mut spv_file)?;
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe { device.create_shader_module(&shader_info, None)? };
        Ok(Shader {
            module,
            reflect_data,
        })
    }
}
