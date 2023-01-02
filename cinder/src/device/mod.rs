mod instance;
mod surface;

pub use self::surface::SurfaceData;
use self::{instance::Instance, surface::Surface};
use crate::{
    profiling::QueryPool,
    resources::{
        buffer::{Buffer, BufferDescription},
        image::{Image, ImageCreateError, ImageDescription},
        pipeline::{
            compute::{ComputePipeline, ComputePipelineDescription},
            graphics::{GraphicsPipeline, GraphicsPipelineDescription},
            PipelineCache,
        },
        sampler::Sampler,
        shader::{Shader, ShaderDescription},
    },
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
    instance: Instance,
    surface: Surface,
    p_device: vk::PhysicalDevice,
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,
    command_pool: vk::CommandPool,
}

impl Device {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let instance = Instance::new(window)?;
        let surface = Surface::new(window, &instance)?;

        let p_devices = unsafe { instance.raw().enumerate_physical_devices() }?;
        let supported_device_data = p_devices
            .into_iter()
            .flat_map(|p_device| {
                unsafe {
                    instance
                        .raw()
                        .get_physical_device_queue_family_properties(p_device)
                }
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
                            unsafe { instance.raw().get_physical_device_properties(p_device) };
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

        let p_device_memory_properties = unsafe {
            instance
                .raw()
                .get_physical_device_memory_properties(p_device)
        };

        let device_extension_names = [
            ash::extensions::khr::Swapchain::name(),
            ash::extensions::khr::DynamicRendering::name(),
            vk::ExtDescriptorIndexingFn::name(),
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
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_variable_descriptor_count(true)
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
        let device = unsafe {
            instance
                .raw()
                .create_device(p_device, &device_create_info, None)
        }?;

        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(queue_family_index),
                None,
            )
        }?;

        Ok(Self {
            instance,
            surface,
            p_device,
            p_device_properties,
            p_device_memory_properties,
            device,
            queue_family_index,
            present_queue,
            command_pool,
        })
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn raw(&self) -> &ash::Device {
        &self.device
    }

    pub fn p_device(&self) -> vk::PhysicalDevice {
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

    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }

    pub fn get_query_pool_results_u32(
        &self,
        query_pool: &QueryPool,
        first_query: u32,
        count: u32,
    ) -> Result<Vec<u32>> {
        let mut ret = Vec::with_capacity((count - first_query) as usize);
        unsafe {
            self.raw().get_query_pool_results(
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
            self.raw().get_query_pool_results(
                query_pool.raw,
                first_query,
                count,
                &mut results,
                vk::QueryResultFlags::TYPE_64,
            )?;
        }
        Ok(results)
    }

    pub fn create_buffer(&self, desc: BufferDescription) -> Result<Buffer> {
        Buffer::create(self, desc)
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Result<Shader> {
        Shader::create(self, desc)
    }

    pub fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescription,
        pipeline_cache: Option<PipelineCache>,
    ) -> Result<GraphicsPipeline> {
        GraphicsPipeline::create(self, pipeline_cache, desc)
    }

    pub fn create_compute_pipeline(
        &self,
        desc: ComputePipelineDescription,
        pipeline_cache: Option<PipelineCache>,
    ) -> Result<ComputePipeline> {
        ComputePipeline::create(self, pipeline_cache, desc)
    }

    pub fn create_sampler(&self) -> Result<Sampler> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            ..Default::default()
        };

        let sampler = unsafe { self.raw().create_sampler(&sampler_info, None) }?;

        Ok(Sampler { raw: sampler })
    }

    pub fn create_image(&self, desc: ImageDescription) -> Result<Image> {
        Image::create(self, self.memopry_properties(), desc)
    }
}
