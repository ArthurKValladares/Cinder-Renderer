use super::{buffer::BindBufferInfo, image::BindImageInfo, shader::ShaderStage};
use crate::cinder::Cinder;
use anyhow::Result;
use ash::vk;
use std::collections::HashMap;

// TODO: This whole layer doesn't quite work

// TODO, maybe could be separate enums, to make bind_buffer, bind_image, etc type-safe
#[derive(Debug, Copy, Clone)]
pub enum BindGroupType {
    ImageSampler,
    UniformBuffer,
    StorageBuffer,
}

impl From<BindGroupType> for vk::DescriptorType {
    fn from(ty: BindGroupType) -> Self {
        match ty {
            BindGroupType::ImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            BindGroupType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            BindGroupType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct BindGroupDesc {
    ty: vk::DescriptorType,
    multiplier: f32,
}

#[derive(Debug)]
struct PoolSizes {
    sizes: Vec<BindGroupDesc>,
}

impl Default for PoolSizes {
    fn default() -> Self {
        Self {
            sizes: vec![
                BindGroupDesc {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    multiplier: 4.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::SAMPLER,
                    multiplier: 0.5,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::SAMPLED_IMAGE,
                    multiplier: 4.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::STORAGE_IMAGE,
                    multiplier: 1.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
                    multiplier: 1.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::STORAGE_TEXEL_BUFFER,
                    multiplier: 1.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    multiplier: 2.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    multiplier: 2.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                    multiplier: 1.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                    multiplier: 1.,
                },
                BindGroupDesc {
                    ty: vk::DescriptorType::INPUT_ATTACHMENT,
                    multiplier: 0.5,
                },
            ],
        }
    }
}

fn create_pool(
    device: &ash::Device,
    pool_sizes: &PoolSizes,
    count: u32,
    flags: vk::DescriptorPoolCreateFlags,
) -> Result<vk::DescriptorPool, vk::Result> {
    let sizes = pool_sizes
        .sizes
        .iter()
        .map(|desc| vk::DescriptorPoolSize {
            ty: desc.ty,
            descriptor_count: (count as f32 * desc.multiplier) as u32,
        })
        .collect::<Vec<_>>();

    let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
        .max_sets(count)
        .pool_sizes(&sizes)
        .flags(flags)
        .build();

    unsafe { device.create_descriptor_pool(&descriptor_pool_info, None) }
}

#[derive(Debug, Default)]
pub struct BindGroupAllocator {
    current_pool: Option<vk::DescriptorPool>,
    descriptor_sizes: PoolSizes,
    used_pools: Vec<vk::DescriptorPool>,
    free_pools: Vec<vk::DescriptorPool>,
}

impl BindGroupAllocator {
    fn grab_pool(&mut self, device: &ash::Device) -> Result<vk::DescriptorPool, vk::Result> {
        if let Some(pool) = self.free_pools.pop() {
            Ok(pool)
        } else {
            create_pool(
                device,
                &self.descriptor_sizes,
                1000, // TODO: arbitrary number
                vk::DescriptorPoolCreateFlags::empty(),
            )
        }
    }

    fn try_allocate_desc_set(
        &mut self,
        device: &ash::Device,
        desc_set_layout: &vk::DescriptorSetLayout,
    ) -> Result<vk::DescriptorSet, vk::Result> {
        if self.current_pool.is_none() {
            let pool = self.grab_pool(device)?;
            self.current_pool = Some(pool);
            self.used_pools.push(pool);
        }

        let current_pool = self.current_pool.unwrap();

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(current_pool)
            .set_layouts(std::slice::from_ref(desc_set_layout));

        let sets = unsafe { device.allocate_descriptor_sets(&desc_alloc_info) }?;

        Ok(sets[0])
    }

    pub fn allocate(
        &mut self,
        device: &ash::Device,
        desc_set_layout: &vk::DescriptorSetLayout,
    ) -> Result<vk::DescriptorSet, vk::Result> {
        let res = self.try_allocate_desc_set(device, desc_set_layout);

        match res {
            Ok(set) => Ok(set),
            Err(result) => {
                if result == vk::Result::ERROR_FRAGMENTED_POOL
                    || result == vk::Result::ERROR_OUT_OF_POOL_MEMORY
                {
                    self.try_allocate_desc_set(device, desc_set_layout)
                } else {
                    Err(result)
                }
            }
        }
    }

    pub fn reset(&mut self, device: &ash::Device) -> Result<()> {
        unsafe {
            for pool in self.used_pools.drain(..) {
                device.reset_descriptor_pool(pool, vk::DescriptorPoolResetFlags::empty())?;
                self.free_pools.push(pool);
            }
        }
        self.current_pool = None;
        Ok(())
    }

    pub fn cleanup(&mut self, device: &ash::Device) {
        unsafe {
            for pool in self.used_pools.drain(..) {
                device.destroy_descriptor_pool(pool, None);
            }
            for pool in self.free_pools.drain(..) {
                device.destroy_descriptor_pool(pool, None);
            }
        }
    }
}

#[derive(Debug)]
pub struct BindGroupLayoutBinding(vk::DescriptorSetLayoutBinding);

impl Eq for BindGroupLayoutBinding {}
impl PartialEq for BindGroupLayoutBinding {
    fn eq(&self, other: &Self) -> bool {
        self.0.binding == other.0.binding
            && self.0.descriptor_type == other.0.descriptor_type
            && self.0.descriptor_count == other.0.descriptor_count
            && self.0.stage_flags == other.0.stage_flags
            && self.0.p_immutable_samplers == other.0.p_immutable_samplers
    }
}

impl std::hash::Hash for BindGroupLayoutBinding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.binding.hash(state);
        self.0.descriptor_type.hash(state);
        self.0.descriptor_count.hash(state);
        self.0.stage_flags.hash(state);
        self.0.p_immutable_samplers.hash(state);
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutInfo {
    bindings: Vec<BindGroupLayoutBinding>,
}

#[derive(Debug, Default)]
pub struct BindGroupLayoutCache {
    cache: HashMap<BindGroupLayoutInfo, vk::DescriptorSetLayout>,
}

impl BindGroupLayoutCache {
    pub fn create_bind_group_layout(
        &mut self,
        device: &ash::Device,
        info: vk::DescriptorSetLayoutCreateInfo,
    ) -> Result<vk::DescriptorSetLayout, vk::Result> {
        let mut bindings = Vec::with_capacity(info.binding_count as usize);
        for _ in 0..info.binding_count {
            bindings.push(BindGroupLayoutBinding(unsafe { *info.p_bindings }));
        }
        bindings.sort_by(|left, right| left.0.binding.partial_cmp(&right.0.binding).unwrap());
        let layout_info = BindGroupLayoutInfo { bindings };
        if let Some(layout) = self.cache.get(&layout_info) {
            Ok(*layout)
        } else {
            let layout = unsafe { device.create_descriptor_set_layout(&info, None) }?;
            self.cache.insert(layout_info, layout);
            Ok(layout)
        }
    }

    pub fn cleanup(&mut self, device: &ash::Device) {
        unsafe {
            for (_, layout) in self.cache.drain() {
                device.destroy_descriptor_set_layout(layout, None);
            }
        }
    }
}

pub struct BindGroupLayout {
    pub layout: vk::DescriptorSetLayout,
}

#[derive(Debug, Default)]
pub struct BindGroupLayoutBuilder {
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
}

impl BindGroupLayoutBuilder {
    pub fn bind_buffer(
        mut self,
        binding: u32,
        ty: BindGroupType,
        shader_stage: ShaderStage,
    ) -> Self {
        let new_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_type(ty.into())
            .descriptor_count(1)
            .stage_flags(shader_stage.into())
            .build();
        self.bindings.push(new_binding);

        self
    }

    pub fn bind_image(
        mut self,
        binding: u32,
        ty: BindGroupType,
        shader_stage: ShaderStage,
    ) -> Self {
        let new_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_type(ty.into())
            .descriptor_count(1)
            .stage_flags(shader_stage.into())
            .build();
        self.bindings.push(new_binding);

        self
    }

    pub fn build(self, cinder: &mut Cinder) -> Result<BindGroupLayout> {
        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&self.bindings)
            .build();

        let layout = cinder.create_descriptor_set_layout(layout_info)?;

        Ok(BindGroupLayout { layout })
    }
}

// TODO: temp, just testing it out
pub struct BindGroupSet {
    pub set: vk::DescriptorSet,
}

impl BindGroupSet {
    pub fn allocate(cinder: &mut Cinder, layout: &BindGroupLayout) -> Result<Self> {
        let set = cinder.create_descriptor_set(&layout.layout)?;
        Ok(Self { set })
    }
}

#[derive(Debug, Default)]
pub struct BindGroupWriteBuilder {
    writes: Vec<vk::WriteDescriptorSet>,
}

impl BindGroupWriteBuilder {
    pub fn bind_buffer(
        mut self,
        binding: u32,
        buffer_info: &BindBufferInfo,
        ty: BindGroupType,
    ) -> Self {
        let new_write = vk::WriteDescriptorSet::builder()
            .descriptor_type(ty.into())
            .buffer_info(std::slice::from_ref(&buffer_info.0))
            .dst_binding(binding)
            .build();
        self.writes.push(new_write);

        self
    }

    pub fn bind_image(
        mut self,
        binding: u32,
        image_info: &BindImageInfo,
        ty: BindGroupType,
    ) -> Self {
        let new_write = vk::WriteDescriptorSet::builder()
            .descriptor_type(ty.into())
            .image_info(std::slice::from_ref(&image_info.0))
            .dst_binding(binding)
            .build();
        self.writes.push(new_write);

        self
    }

    pub fn update(mut self, cinder: &Cinder, set: &BindGroupSet) {
        for write in &mut self.writes {
            write.dst_set = set.set;
        }

        unsafe {
            cinder.device().update_descriptor_sets(&self.writes, &[]);
        }
    }
}
