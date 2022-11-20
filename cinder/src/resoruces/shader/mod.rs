use anyhow::Result;
use ash::vk;
use rust_shader_tools::{ReflectShaderStageFlags, ShaderData};
use std::io::Cursor;

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

impl From<ReflectShaderStageFlags> for ShaderStage {
    fn from(flags: ReflectShaderStageFlags) -> Self {
        match flags {
            ReflectShaderStageFlags::VERTEX => ShaderStage::Vertex,
            ReflectShaderStageFlags::FRAGMENT => ShaderStage::Fragment,
            _ => panic!("Shader stage not yet supported"),
        }
    }
}

pub struct ShaderDescription {
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

    pub fn stage(&self) -> ShaderStage {
        self.reflect_data.stage().into()
    }
}
