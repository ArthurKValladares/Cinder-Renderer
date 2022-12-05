use super::BindGroupType;
use crate::{
    cinder::Cinder,
    resoruces::{buffer::BindBufferInfo, image::BindImageInfo, shader::ShaderStage},
};
use anyhow::Result;
use ash::vk;

pub struct NewBindGroupPool(vk::DescriptorPool);

impl NewBindGroupPool {
    pub fn new(cinder: &Cinder) -> Result<Self> {
        let descriptor_count = cinder.max_bindless_descriptor_count();
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count,
            },
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(descriptor_count * pool_sizes.len() as u32)
            .pool_sizes(&pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .build();

        let pool = unsafe {
            cinder
                .device()
                .create_descriptor_pool(&descriptor_pool_info, None)?
        };

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

pub fn bindless_bind_group_flags() -> vk::DescriptorBindingFlags {
    vk::DescriptorBindingFlags::PARTIALLY_BOUND
        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
}

pub struct NewBindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
    pub variable_count: bool,
}

impl NewBindGroupLayout {
    pub fn new(cinder: &Cinder, layout_data: &[BindGroupLayoutData]) -> Result<Self> {
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

        let mut variable_count = false;
        let binding_flags = layout_data
            .iter()
            .map(|data| {
                variable_count |= data
                    .flags
                    .contains(vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT);
                data.flags
            })
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
            cinder
                .device()
                .create_descriptor_set_layout(&layout_info, None)
        }?;

        Ok(Self {
            layout,
            variable_count,
        })
    }
}

#[derive(Debug)]
pub enum BindGroupWriteData {
    Storage(BindBufferInfo),
    Uniform(BindBufferInfo),
    Image(BindImageInfo),
}

#[derive(Debug)]
pub struct BindGroupBindInfo {
    pub dst_binding: u32,
    pub data: BindGroupWriteData,
}

pub struct NewBindGroup(pub vk::DescriptorSet);

impl NewBindGroup {
    pub fn new(
        cinder: &Cinder,
        pool: &NewBindGroupPool,
        layout: &NewBindGroupLayout,
    ) -> Result<Self> {
        let max_binding = cinder.max_bindless_descriptor_count() - 1;

        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[max_binding])
            .build();

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool.0)
            .set_layouts(std::slice::from_ref(&layout.layout));
        let desc_alloc_info = if layout.variable_count {
            desc_alloc_info.push_next(&mut count_info).build()
        } else {
            desc_alloc_info.build()
        };

        let set = unsafe { cinder.device().allocate_descriptor_sets(&desc_alloc_info) }?[0];

        Ok(Self(set))
    }

    pub fn write(&self, cinder: &Cinder, infos: &[BindGroupBindInfo]) {
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
                    BindGroupWriteData::Image(info) => write
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .dst_array_element(info.index)
                        .image_info(std::slice::from_ref(&info.info)),
                };
                write.build()
            })
            .collect::<Vec<_>>();

        unsafe {
            cinder.device().update_descriptor_sets(&writes, &[]);
        }
    }
}
