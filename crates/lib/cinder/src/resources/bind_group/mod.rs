use crate::{
    device::{Device, MAX_BINDLESS_RESOURCES},
    resources::{buffer::BindBufferInfo, image::BindImageInfo, shader::ShaderStage},
};
use anyhow::Result;
use ash::vk;

// TODO, maybe could be separate enums, to make bind_buffer, bind_image, etc type-safe
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
    pub fn new(device: &ash::Device) -> Result<Self> {
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

        Ok(Self(pool))
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.0, None);
        }
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutData {
    pub binding: u32,
    pub ty: BindGroupType,
    pub count: Option<u32>,
    pub shader_stage: ShaderStage,
    pub flags: vk::DescriptorBindingFlags,
}

impl BindGroupLayoutData {
    pub fn new(binding: u32, ty: BindGroupType, shader_stage: ShaderStage) -> Self {
        Self {
            binding,
            ty,
            count: Some(1),
            shader_stage,
            flags: Default::default(),
        }
    }

    pub fn new_bindless(binding: u32, ty: BindGroupType, shader_stage: ShaderStage) -> Self {
        Self {
            binding,
            ty,
            count: None,
            shader_stage,
            flags: bindless_bind_group_flags(),
        }
    }
}

pub fn bindless_bind_group_flags() -> vk::DescriptorBindingFlags {
    vk::DescriptorBindingFlags::PARTIALLY_BOUND
        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
}

#[repr(C)]
#[derive(Debug)]
pub struct BindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
}

impl BindGroupLayout {
    pub fn new(device: &Device, layout_data: &[BindGroupLayoutData]) -> Result<Self> {
        let bindings = layout_data
            .iter()
            .map(|data| {
                vk::DescriptorSetLayoutBinding::builder()
                    .descriptor_type(data.ty.into())
                    .binding(data.binding)
                    .descriptor_count(data.count.unwrap_or(MAX_BINDLESS_RESOURCES))
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

        Ok(Self { layout })
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(vk::ObjectType::DESCRIPTOR_SET_LAYOUT, self.layout, name);
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        unsafe { device.destroy_descriptor_set_layout(self.layout, None) }
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
    pub dst_binding: u32,
    pub data: BindGroupWriteData,
}

pub struct BindGroup(pub vk::DescriptorSet);

impl BindGroup {
    pub fn new(
        device: &Device,
        layouts: &[BindGroupLayout],
        descriptor_counts: &[u32],
    ) -> Result<Self> {
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(descriptor_counts)
            .build();

        let set_layouts = layouts
            .iter()
            .map(|layout| layout.layout)
            .collect::<Vec<_>>();

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(device.bind_group_pool.0)
            .set_layouts(&set_layouts)
            .push_next(&mut count_info)
            .build();

        let set = unsafe { device.raw().allocate_descriptor_sets(&desc_alloc_info) }?[0];

        Ok(Self(set))
    }

    pub(crate) fn set_name(&self, device: &Device, name: &str) {
        device.set_name(vk::ObjectType::DESCRIPTOR_SET, self.0, name);
    }
}
