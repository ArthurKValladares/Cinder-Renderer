use anyhow::Result;
use ash::vk;

use crate::cinder::Cinder;

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

pub struct NewBindGroupLayout(vk::DescriptorSetLayout);

impl NewBindGroupLayout {
    pub fn new(cinder: &Cinder) -> Result<Self> {
        let uniform = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        let storage = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        let image = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(cinder.max_bindless_descriptor_count())
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let bindings = [uniform, storage, image];

        let bindless_flags = vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND;

        let mut extended_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&[
                vk::DescriptorBindingFlags::empty(),
                vk::DescriptorBindingFlags::empty(),
                bindless_flags,
            ])
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

        Ok(Self(layout))
    }
}

pub struct NewBindGroup(vk::DescriptorSet);

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
            .set_layouts(std::slice::from_ref(&layout.0))
            .push_next(&mut count_info)
            .build();

        let set = unsafe { cinder.device().allocate_descriptor_sets(&desc_alloc_info) }?[0];

        Ok(Self(set))
    }
}
