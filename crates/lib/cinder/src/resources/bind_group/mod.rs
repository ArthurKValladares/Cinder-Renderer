use crate::{
    device::{set_object_name, Device, Instance, MAX_BINDLESS_RESOURCES},
    resources::{buffer::BindBufferInfo, image::BindImageInfo, shader::ShaderStage},
};
use anyhow::Result;
use ash::vk;
use std::collections::BTreeMap;

#[derive(Debug, Copy, Clone)]
pub enum BindGroupType {
    ImageSampler,
    StorageImage,
    UniformBuffer,
    StorageBuffer,
}

impl From<BindGroupType> for vk::DescriptorType {
    fn from(ty: BindGroupType) -> Self {
        match ty {
            BindGroupType::ImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            BindGroupType::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
            BindGroupType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            BindGroupType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        }
    }
}

pub struct BindGroupPool(pub(crate) vk::DescriptorPool);

impl BindGroupPool {
    pub fn new(instance: &Instance, device: &ash::Device) -> Result<Self> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_BINDLESS_RESOURCES,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_BINDLESS_RESOURCES,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: MAX_BINDLESS_RESOURCES,
            },
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(MAX_BINDLESS_RESOURCES * pool_sizes.len() as u32)
            .pool_sizes(&pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .build();

        let pool = unsafe { device.create_descriptor_pool(&descriptor_pool_info, None)? };
        set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::DESCRIPTOR_POOL,
            pool,
            "Descriptor Pool",
        );

        Ok(Self(pool))
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.0, None);
        }
    }
}

pub type BindGroupSet = u32;

#[derive(Debug)]
pub struct BindGroupBindingData {
    pub binding: u32,
    pub ty: BindGroupType,
    pub count: u32,
    pub shader_stage: ShaderStage,
    pub flags: vk::DescriptorBindingFlags,
}

impl BindGroupBindingData {
    pub fn new(binding: u32, ty: BindGroupType, count: u32, shader_stage: ShaderStage) -> Self {
        Self {
            binding,
            ty,
            count,
            shader_stage,
            flags: if count > 1 {
                bindless_bind_group_flags()
            } else {
                Default::default()
            },
        }
    }
}

pub fn bindless_bind_group_flags() -> vk::DescriptorBindingFlags {
    vk::DescriptorBindingFlags::PARTIALLY_BOUND
        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
}

#[repr(transparent)]
#[derive(Debug)]
pub struct BindGroupLayout(pub vk::DescriptorSetLayout);

impl BindGroupLayout {
    pub fn new(device: &Device, layout_data: &[BindGroupBindingData]) -> Result<Self> {
        let bindings = layout_data
            .iter()
            .map(|data| {
                vk::DescriptorSetLayoutBinding::builder()
                    .descriptor_type(data.ty.into())
                    .binding(data.binding)
                    .descriptor_count(data.count)
                    .stage_flags(data.shader_stage.into())
                    .build()
            })
            .collect::<Vec<_>>();

        let binding_flags = layout_data
            .iter()
            .map(|data| data.flags)
            .collect::<Vec<_>>();
        let mut extended_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&binding_flags)
            .build();

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .push_next(&mut extended_info)
            .build();

        let layout = unsafe {
            device
                .raw()
                .create_descriptor_set_layout(&layout_info, None)
        }?;

        Ok(Self(layout))
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(vk::ObjectType::DESCRIPTOR_SET_LAYOUT, self.0, name);
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_descriptor_set_layout(self.0, None) }
    }
}

#[derive(Debug)]
pub enum BindGroupWriteData {
    Storage(BindBufferInfo),
    Uniform(BindBufferInfo),
    SampledImage(BindImageInfo),
    StorageImage(BindImageInfo),
}

#[derive(Debug)]
pub struct BindGroupBindInfo {
    pub group: BindGroup,
    pub dst_binding: u32,
    pub data: BindGroupWriteData,
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct BindGroup(pub vk::DescriptorSet);

impl BindGroup {
    pub fn new(device: &Device, bind_group_data: &BindGroupData) -> Result<Self> {
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(std::slice::from_ref(&bind_group_data.count))
            .build();

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(device.bind_group_pool.0)
            .set_layouts(std::slice::from_ref(&bind_group_data.layout.0))
            .push_next(&mut count_info)
            .build();

        let set = unsafe { device.raw().allocate_descriptor_sets(&desc_alloc_info) }?[0];

        Ok(Self(set))
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(vk::ObjectType::DESCRIPTOR_SET, self.0, name);
    }
}

#[derive(Debug)]
pub struct BindGroupData {
    pub count: u32,
    pub layout: BindGroupLayout,
}

impl BindGroupData {
    pub fn destroy(&self, device: &ash::Device) {
        self.layout.destroy(device);
    }
}

#[derive(Debug, Default)]
pub struct BindGroupMap {
    pub map: BTreeMap<usize, BindGroupData>,
}

impl BindGroupMap {
    pub fn destroy(&self, device: &ash::Device) {
        for layout in self.map.values() {
            layout.destroy(device);
        }
    }
}
