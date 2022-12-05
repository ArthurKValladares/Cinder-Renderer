use super::{
    bind_group::{BindGroupLayoutData, BindGroupType},
    pipeline::push_constant::PushConstant,
};
use anyhow::Result;
use ash::vk;
use rust_shader_tools::{
    is_runtime_array, ReflectDescriptorType, ReflectShaderStageFlags, ShaderData,
};
use std::{collections::BTreeMap, io::Cursor};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("{0}")]
    ReflectionError(&'static str),
}

pub struct ShaderDescription {
    pub bytes: &'static [u8],
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
    pub(crate) reflect_data: ShaderData,
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

    pub fn push_constants(&self) -> Result<Vec<PushConstant>> {
        Ok(self
            .reflect_data
            .module()
            .enumerate_push_constant_blocks(None)
            .map_err(ShaderError::ReflectionError)?
            .iter()
            .map(|block| {
                assert!(!block.members.is_empty());
                let offset = block.members[0].offset;
                let mut size = 0;
                for member in &block.members {
                    size += member.size;
                }
                PushConstant {
                    stage: self.stage(),
                    offset,
                    size,
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn bind_group_layouts(&self) -> Result<BTreeMap<u32, Vec<BindGroupLayoutData>>> {
        let shader_stage = self.stage();
        Ok(self
            .reflect_data
            .module()
            .enumerate_descriptor_sets(None)
            .map_err(ShaderError::ReflectionError)?
            .iter()
            .map(|set| {
                let data = set
                    .bindings
                    .iter()
                    .map(|reflect_binding| {
                        let ty = match reflect_binding.descriptor_type {
                            ReflectDescriptorType::CombinedImageSampler => {
                                BindGroupType::ImageSampler
                            }
                            ReflectDescriptorType::UniformBuffer => BindGroupType::UniformBuffer,
                            ReflectDescriptorType::StorageBuffer => BindGroupType::StorageBuffer,
                            _ => {
                                // TODO: need a better way to handle returning errors from here later
                                unreachable!()
                            }
                        };
                        let array =
                            if let Some(type_description) = &reflect_binding.type_description {
                                is_runtime_array(type_description.op)
                            } else {
                                false
                            };
                        if array {
                            BindGroupLayoutData::new_bindless(
                                reflect_binding.binding,
                                ty,
                                shader_stage,
                            )
                        } else {
                            BindGroupLayoutData::new(reflect_binding.binding, ty, shader_stage)
                        }
                    })
                    .collect::<Vec<_>>();
                (set.set, data)
            })
            .collect::<BTreeMap<_, _>>())
    }
}
