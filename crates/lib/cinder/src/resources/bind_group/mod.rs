use crate::{
    device::Device,
    resources::{buffer::BindBufferInfo, image::BindImageInfo, shader::ShaderStage},
};
use anyhow::Result;
use ash::vk;

use super::{pipeline::graphics::GraphicsPipeline, ResourceHandle};

pub const MAX_BINDLESS_RESOURCES: u32 = 16536;

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

pub struct BindGroupPool(vk::DescriptorPool);

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
}

#[derive(Debug)]
pub struct BindGroupLayoutData {
    pub binding: u32,
    pub ty: BindGroupType,
    pub count: u32,
    pub shader_stage: ShaderStage,
    pub flags: vk::DescriptorBindingFlags,
}

impl BindGroupLayoutData {
    pub fn new(binding: u32, ty: BindGroupType, shader_stage: ShaderStage) -> Self {
        Self {
            binding,
            ty,
            count: 1,
            shader_stage,
            flags: Default::default(),
        }
    }

    pub fn new_bindless(binding: u32, ty: BindGroupType, shader_stage: ShaderStage) -> Self {
        Self {
            binding,
            ty,
            count: MAX_BINDLESS_RESOURCES,
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

pub struct BindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
}

impl BindGroupLayout {
    pub fn new(device: &ash::Device, layout_data: &[BindGroupLayoutData]) -> Result<Self> {
        let bindings = layout_data
            .iter()
            .map(|data| {
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(data.binding)
                    .descriptor_type(data.ty.into())
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

        let layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }?;

        Ok(Self { layout })
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
        handle: ResourceHandle<GraphicsPipeline>,
        variable_count: bool,
    ) -> Result<Self> {
        // TODO: Get rid of unwrap later
        let layout = &device
            .get_graphics_pipeline(handle)
            .unwrap()
            .common
            .bind_group_layouts()[0];

        let max_binding = MAX_BINDLESS_RESOURCES - 1;

        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[max_binding])
            .build();

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(device.bind_group_pool.0)
            .set_layouts(std::slice::from_ref(&layout.layout));
        let desc_alloc_info = if variable_count {
            desc_alloc_info.push_next(&mut count_info).build()
        } else {
            desc_alloc_info.build()
        };

        let set = unsafe { device.raw().allocate_descriptor_sets(&desc_alloc_info) }?[0];

        Ok(Self(set))
    }

    pub fn write(&self, device: &Device, infos: &[BindGroupBindInfo]) {
        let writes = infos
            .iter()
            .map(|info| {
                let mut write = vk::WriteDescriptorSet::builder()
                    .dst_set(self.0)
                    .dst_binding(info.dst_binding);
                write = match &info.data {
                    BindGroupWriteData::Uniform(buffer_info) => write
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .buffer_info(std::slice::from_ref(&buffer_info.0)),
                    BindGroupWriteData::Storage(buffer_info) => write
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(std::slice::from_ref(&buffer_info.0)),
                    BindGroupWriteData::SampledImage(info) => write
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .dst_array_element(info.index)
                        .image_info(std::slice::from_ref(&info.info)),
                    BindGroupWriteData::StorageImage(info) => write
                        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                        .dst_array_element(info.index)
                        .image_info(std::slice::from_ref(&info.info)),
                };
                write.build()
            })
            .collect::<Vec<_>>();

        unsafe {
            device.raw().update_descriptor_sets(&writes, &[]);
        }
    }
}
