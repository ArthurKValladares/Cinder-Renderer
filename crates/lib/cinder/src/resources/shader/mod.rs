use crate::device::Device;

use super::{
    bind_group::{BindGroupLayoutData, BindGroupType},
    pipeline::push_constant::PushConstant,
};
use anyhow::Result;
use ash::vk;
use rust_shader_tools::{
    is_runtime_array, ReflectDescriptorType, ReflectEntryPointLocalSize, ReflectShaderStageFlags,
    ShaderData,
};
use std::{collections::BTreeMap, io::Cursor};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl From<ShaderStage> for vk::ShaderStageFlags {
    fn from(stage: ShaderStage) -> Self {
        match stage {
            ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
            ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
            ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
        }
    }
}

impl From<ReflectShaderStageFlags> for ShaderStage {
    fn from(flags: ReflectShaderStageFlags) -> Self {
        match flags {
            ReflectShaderStageFlags::VERTEX => ShaderStage::Vertex,
            ReflectShaderStageFlags::FRAGMENT => ShaderStage::Fragment,
            ReflectShaderStageFlags::COMPUTE => ShaderStage::Compute,
            _ => panic!("Shader stage not yet supported."),
        }
    }
}

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("{0}")]
    ReflectionError(&'static str),
    #[error("Unsupported descriptor type: {0:#?}")]
    UnsupportedDescriptorType(ReflectDescriptorType),
}

#[derive(Default, Copy, Clone)]
pub struct ShaderDesc {
    pub name: Option<&'static str>,
}

pub struct Shader {
    pub(crate) module: vk::ShaderModule,
    pub reflect_data: ShaderData,
    pub desc: ShaderDesc,
}

impl Shader {
    pub(crate) fn create(device: &Device, bytes: &[u8], desc: ShaderDesc) -> Result<Self> {
        let reflect_data = ShaderData::from_spv(bytes)?;
        let mut spv_file = Cursor::new(bytes);
        let code = ash::util::read_spv(&mut spv_file)?;
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe { device.raw().create_shader_module(&shader_info, None)? };

        if let Some(name) = desc.name {
            device.set_name(vk::ObjectType::SHADER_MODULE, module, name);
        }

        Ok(Shader {
            module,
            reflect_data,
            desc,
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

    pub fn bind_group_layouts(
        &self,
        p_device_descriptor_indexing_properties: vk::PhysicalDeviceDescriptorIndexingProperties,
    ) -> Result<BTreeMap<u32, Vec<BindGroupLayoutData>>, ShaderError> {
        let shader_stage = self.stage();
        self.reflect_data
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
                                Some(BindGroupType::ImageSampler)
                            }
                            ReflectDescriptorType::UniformBuffer => {
                                Some(BindGroupType::UniformBuffer)
                            }
                            ReflectDescriptorType::StorageBuffer => {
                                Some(BindGroupType::StorageBuffer)
                            }
                            ReflectDescriptorType::StorageImage => {
                                Some(BindGroupType::StorageImage)
                            }
                            _ => None,
                        };
                        if let Some(ty) = ty {
                            let array =
                                if let Some(type_description) = &reflect_binding.type_description {
                                    is_runtime_array(type_description.op)
                                } else {
                                    false
                                };
                            let count = if array {
                                match ty {
                                BindGroupType::ImageSampler => {
                                    p_device_descriptor_indexing_properties
                                        .max_per_stage_descriptor_update_after_bind_samplers
                                }
                                BindGroupType::StorageImage => {
                                    p_device_descriptor_indexing_properties
                                        .max_per_stage_descriptor_update_after_bind_storage_images
                                }
                                BindGroupType::UniformBuffer => {
                                    p_device_descriptor_indexing_properties
                                        .max_per_stage_descriptor_update_after_bind_uniform_buffers
                                }
                                BindGroupType::StorageBuffer => {
                                    p_device_descriptor_indexing_properties
                                        .max_per_stage_descriptor_update_after_bind_storage_buffers
                                }
                            }
                            } else {
                                1
                            };
                            Ok(BindGroupLayoutData::new(
                                reflect_binding.binding,
                                ty,
                                count,
                                shader_stage,
                            ))
                        } else {
                            Err(ShaderError::UnsupportedDescriptorType(
                                reflect_binding.descriptor_type,
                            ))
                        }
                    })
                    .collect::<Result<Vec<_>, ShaderError>>()?;
                Ok((set.set, data))
            })
            .collect::<Result<BTreeMap<_, _>, ShaderError>>()
    }

    pub fn local_size(&self) -> ReflectEntryPointLocalSize {
        self.reflect_data.module().enumerate_entry_points().unwrap()[0].local_size
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_shader_module(self.module, None);
        }
    }
}
