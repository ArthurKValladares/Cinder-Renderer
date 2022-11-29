use std::ops::Deref;

use crate::{
    instance::Instance, profiling::QueryPool, resoruces::image::ImageCreateError, surface::Surface,
};
use anyhow::Result;
use ash::vk;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::KhrPortabilitySubsetFn;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceInitError {
    #[error("No suitable device found")]
    NoSuitableDevice,
    #[error(transparent)]
    ImageCreateError(#[from] ImageCreateError),
}

pub struct Device {
    p_device: vk::PhysicalDevice,
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,
}

impl Device {
    pub fn new(instance: &Instance, surface: &Surface) -> Result<Self> {
        let p_devices = unsafe { instance.enumerate_physical_devices() }?;
        let supported_device_data = p_devices
            .into_iter()
            .flat_map(|p_device| {
                unsafe { instance.get_physical_device_queue_family_properties(p_device) }
                    .iter()
                    .enumerate()
                    .filter_map(|(index, info)| {
                        let supports_graphic_and_surface =
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && unsafe {
                                    surface.surface_loader.get_physical_device_surface_support(
                                        p_device,
                                        index as u32,
                                        surface.surface,
                                    )
                                }
                                .unwrap_or(false);
                        if supports_graphic_and_surface {
                            let properties =
                                unsafe { instance.get_physical_device_properties(p_device) };
                            Some((p_device, index as u32, properties))
                        } else {
                            None
                        }
                    })
                    .next()
            })
            .collect::<Vec<_>>();
        let (p_device, queue_family_index, p_device_properties) = supported_device_data
            .into_iter()
            .rev()
            .max_by_key(|(_, _, properties)| match properties.device_type {
                vk::PhysicalDeviceType::INTEGRATED_GPU => 200,
                vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 1,
                _ => 0,
            })
            .ok_or(DeviceInitError::NoSuitableDevice)?;

        let p_device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(p_device) };

        let device_extension_names = [
            ash::extensions::khr::Swapchain::name(),
            ash::extensions::khr::DynamicRendering::name(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            KhrPortabilitySubsetFn::name(),
        ];
        let device_extension_names_raw: Vec<*const i8> = device_extension_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut scalar_block = vk::PhysicalDeviceScalarBlockLayoutFeaturesEXT::builder()
            .scalar_block_layout(true)
            .build();
        let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeaturesEXT::builder()
            .descriptor_binding_partially_bound(true)
            .runtime_descriptor_array(true)
            .build();
        let mut dynamic_rendering = vk::PhysicalDeviceDynamicRenderingFeatures::builder()
            .dynamic_rendering(true)
            .build();
        let mut features = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut scalar_block)
            .push_next(&mut descriptor_indexing)
            .push_next(&mut dynamic_rendering)
            .build();

        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .push_next(&mut features)
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw);
        let device = unsafe { instance.create_device(p_device, &device_create_info, None) }?;

        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        Ok(Self {
            p_device,
            p_device_properties,
            p_device_memory_properties,
            device,
            queue_family_index,
            present_queue,
        })
    }

    pub(crate) fn p_device(&self) -> vk::PhysicalDevice {
        self.p_device
    }

    pub fn properties(&self) -> &vk::PhysicalDeviceProperties {
        &self.p_device_properties
    }

    pub fn memopry_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.p_device_memory_properties
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }

    pub fn get_query_pool_results_u32(
        &self,
        query_pool: &QueryPool,
        first_query: u32,
        count: u32,
    ) -> Result<Vec<u32>> {
        let mut ret = Vec::with_capacity((count - first_query) as usize);
        unsafe {
            self.get_query_pool_results(
                query_pool.raw,
                first_query,
                count,
                &mut ret,
                vk::QueryResultFlags::empty(),
            )?;
        }
        Ok(ret)
    }

    pub fn get_query_pool_results_u64(
        &self,
        query_pool: &QueryPool,
        first_query: u32,
        count: u32,
    ) -> Result<Vec<u64>> {
        let query_count = (count - first_query) as usize;
        let mut results = vec![0; query_count];
        unsafe {
            self.get_query_pool_results(
                query_pool.raw,
                first_query,
                count,
                &mut results,
                vk::QueryResultFlags::TYPE_64,
            )?;
        }
        Ok(results)
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
