mod instance;
mod surface;

pub use self::{instance::Extension, surface::SurfaceData};
use self::{instance::Instance, surface::Surface};
use crate::{
    context::ContextShared,
    profiling::QueryPool,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupPool, BindGroupWriteData},
        buffer::{Buffer, BufferDescription},
        image::{Image, ImageDescription, ImageError},
        pipeline::{
            compute::{ComputePipeline, ComputePipelineDescription},
            graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        },
        sampler::{Sampler, SamplerDescription},
        shader::{Shader, ShaderDesc},
    },
    Resolution,
};
use anyhow::Result;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::KhrPortabilitySubsetFn;
use ash::{extensions::khr::DynamicRendering, vk};
use math::{rect::Rect2D, size::Size2D};
use resource_manager::{ResourceHandle, ResourcePool};
use thiserror::Error;
use util::size_of_slice;

fn max_bindless_descriptor_count_inner(p_device_properties: &vk::PhysicalDeviceProperties) -> u32 {
    pub const RESERVED_DESCRIPTOR_COUNT: u32 = 32;

    (512 * 1024).min(
        p_device_properties
            .limits
            .max_per_stage_descriptor_sampled_images
            - RESERVED_DESCRIPTOR_COUNT,
    )
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("No suitable device found")]
    NoSuitableDevice,
    #[error(transparent)]
    ImageCreateError(#[from] ImageError),
    #[error("Invalid pipeline handle")]
    InvalidPipelineHandle,
}

#[derive(Debug, Default)]
pub struct DeviceDescription<'a> {
    pub required_extensions: &'a [Extension],
}

pub struct Device {
    p_device: vk::PhysicalDevice,
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,
    command_pool: vk::CommandPool,
    surface: Surface,
    instance: Instance,
    pub(crate) pipeline_cache: vk::PipelineCache,
    pub(crate) bind_group_pool: BindGroupPool,
    pub(crate) surface_data: SurfaceData,
    // TODO: Probably will have better syncronization in the future, not pub
    pub(crate) present_complete_semaphore: vk::Semaphore,
    pub(crate) rendering_complete_semaphore: vk::Semaphore,
    pub(crate) draw_commands_reuse_fence: vk::Fence,
    pub(crate) setup_commands_reuse_fence: vk::Fence,
    // TODO: Probably some place to shove extensions
    dynamic_rendering: DynamicRendering,
    // TODO: Experimenting with some resource handling stuff in Device. maybe should be separate
    pipelines: ResourcePool<GraphicsPipeline>,
    shaders: ResourcePool<Shader>,
}

impl Device {
    pub fn new(window: &winit::window::Window, desc: DeviceDescription) -> Result<Self> {
        let instance = Instance::new(window, desc.required_extensions)?;
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
            .ok_or(DeviceError::NoSuitableDevice)?;

        let p_device_memory_properties = unsafe {
            instance
                .raw()
                .get_physical_device_memory_properties(p_device)
        };

        let device_extension_names = [
            ash::extensions::khr::Swapchain::name(),
            ash::extensions::khr::DynamicRendering::name(),
            vk::ExtDescriptorIndexingFn::name(),
            unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"VK_KHR_depth_stencil_resolve\0")
            },
            unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"VK_KHR_create_renderpass2\0")
            },
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
        //instance::debug::set_object_name(
        //    instance.debug(),
        //    device.handle(),
        //    vk::ObjectType::INSTANCE,
        //    instance.raw().handle(),
        //    "instance",
        //);
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::DEVICE,
            device.handle(),
            "device",
        );
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::PHYSICAL_DEVICE,
            p_device,
            "physical device",
        );

        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::QUEUE,
            present_queue,
            "present queue",
        );

        let command_pool = unsafe {
            device.create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(queue_family_index),
                None,
            )
        }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::COMMAND_POOL,
            command_pool,
            "command pool",
        );

        let ci = vk::PipelineCacheCreateInfo::builder().build();
        let pipeline_cache = unsafe { device.create_pipeline_cache(&ci, None)? };
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::PIPELINE_CACHE,
            pipeline_cache,
            "pipeline cache",
        );
        let bind_group_pool = BindGroupPool::new(
            &device,
            max_bindless_descriptor_count_inner(&p_device_properties),
        )?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::DESCRIPTOR_POOL,
            bind_group_pool.0,
            "descriptor pool",
        );

        let window_size = window.inner_size();
        let surface_data = surface.get_data(
            p_device,
            Resolution {
                width: window_size.width,
                height: window_size.height,
            },
            false,
        )?;

        // TODO: Figure out sync story
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::SEMAPHORE,
            present_complete_semaphore,
            "present complete semaphore",
        );

        let rendering_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::SEMAPHORE,
            rendering_complete_semaphore,
            "rendering complete semaphore",
        );

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::FENCE,
            draw_commands_reuse_fence,
            "draw commands reuse fence",
        );

        let setup_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::FENCE,
            setup_commands_reuse_fence,
            "setup commands reuse fence",
        );

        let dynamic_rendering = DynamicRendering::new(instance.raw(), &device);
        Ok(Self {
            instance,
            surface,
            surface_data,
            p_device,
            p_device_properties,
            p_device_memory_properties,
            device,
            queue_family_index,
            present_queue,
            command_pool,
            pipeline_cache,
            bind_group_pool,
            dynamic_rendering,
            present_complete_semaphore,
            rendering_complete_semaphore,
            draw_commands_reuse_fence,
            setup_commands_reuse_fence,
            pipelines: Default::default(),
            shaders: Default::default(),
        })
    }

    pub(crate) fn set_name(
        &self,
        object_type: vk::ObjectType,
        object: impl vk::Handle,
        name: &str,
    ) {
        instance::debug::set_object_name(
            self.instance.debug(),
            self.device.handle(),
            object_type,
            object,
            name,
        )
    }

    pub fn begin_context_label(&self, context: &ContextShared, name: &str, color: [f32; 4]) {
        instance::debug::cmd_begin_label(
            self.instance.debug(),
            context.command_buffer,
            name,
            color,
        );
    }

    pub fn end_context_label(&self, context: &ContextShared) {
        instance::debug::cmd_end_label(self.instance.debug(), context.command_buffer);
    }

    pub fn insert_context_label(&self, context: &ContextShared, name: &str, color: [f32; 4]) {
        instance::debug::cmd_insert_label(
            self.instance.debug(),
            context.command_buffer,
            name,
            color,
        );
    }

    pub fn begin_queue_label(&self, name: &str, color: [f32; 4]) {
        instance::debug::queue_begin_label(self.instance.debug(), self.present_queue, name, color);
    }

    pub fn end_queue_label(&self) {
        instance::debug::queue_end_label(self.instance.debug(), self.present_queue);
    }

    pub fn insert_queue_label(&self, name: &str, color: [f32; 4]) {
        instance::debug::queue_insert_label(self.instance.debug(), self.present_queue, name, color);
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

    pub fn dynamic_rendering(&self) -> &DynamicRendering {
        &self.dynamic_rendering
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

    pub fn create_buffer(&self, size: u64, desc: BufferDescription) -> Result<Buffer> {
        Buffer::create(self, size, desc)
    }

    pub fn create_buffer_with_data<T: Copy>(
        &self,
        data: &[T],
        desc: BufferDescription,
    ) -> Result<Buffer> {
        let size = size_of_slice(data);
        let buffer = self.create_buffer(size, desc)?;
        buffer.mem_copy(0, data)?;
        Ok(buffer)
    }

    pub fn create_image(&self, size: Size2D<u32>, desc: ImageDescription) -> Result<Image> {
        Image::create(self, size, desc)
    }

    pub fn create_shader(
        &mut self,
        bytes: &[u8],
        desc: ShaderDesc,
    ) -> Result<ResourceHandle<Shader>> {
        Ok(self.shaders.insert(Shader::create(self, bytes, desc)?))
    }

    pub(crate) fn get_shader(&self, handle: ResourceHandle<Shader>) -> Option<&Shader> {
        self.shaders.get(handle)
    }

    pub fn create_graphics_pipeline(
        &mut self,
        vertex_shader: ResourceHandle<Shader>,
        fragment_shader: ResourceHandle<Shader>,
        desc: GraphicsPipelineDescription,
    ) -> Result<ResourceHandle<GraphicsPipeline>> {
        Ok(self.pipelines.insert(GraphicsPipeline::create(
            self,
            vertex_shader,
            fragment_shader,
            desc,
        )?))
    }

    // TODO: Error handling
    pub fn recreate_graphics_pipeline(&mut self, handle: ResourceHandle<GraphicsPipeline>) {
        let new = self.pipelines.get(handle).map(|old| {
            GraphicsPipeline::create(
                self,
                old.vertex_shader_handle,
                old.fragment_shader_handle,
                old.desc,
            )
            .unwrap()
        });

        if let Some(new) = new {
            self.pipelines.replace(handle, new);
        } else {
            // TODO: error
        }
    }

    pub(crate) fn get_graphics_pipeline(
        &self,
        handle: ResourceHandle<GraphicsPipeline>,
    ) -> Option<&GraphicsPipeline> {
        self.pipelines.get(handle)
    }

    pub fn create_compute_pipeline(
        &self,
        shader: Shader,
        desc: ComputePipelineDescription,
    ) -> Result<ComputePipeline> {
        ComputePipeline::create(self, shader, desc)
    }

    pub fn create_sampler(&self, device: &Device, desc: SamplerDescription) -> Result<Sampler> {
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

        if let Some(name) = desc.name {
            device.set_name(vk::ObjectType::SAMPLER, sampler, name);
        }

        Ok(Sampler { raw: sampler })
    }

    pub fn present_complete_semaphore(&self) -> vk::Semaphore {
        self.present_complete_semaphore
    }

    pub fn setup_fence(&self) -> vk::Fence {
        self.setup_commands_reuse_fence
    }

    pub fn surface_data(&self) -> &SurfaceData {
        &self.surface_data
    }

    pub fn surface_rect(&self) -> Rect2D<i32, u32> {
        Rect2D::from_width_height(
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
        )
    }

    pub fn write_bind_group(
        &self,
        handle: ResourceHandle<GraphicsPipeline>,
        infos: &[BindGroupBindInfo],
    ) -> Result<(), DeviceError> {
        if let Some(pipeline) = self.get_graphics_pipeline(handle) {
            let writes = infos
                .iter()
                .map(|info| {
                    let mut write = vk::WriteDescriptorSet::builder()
                        .dst_set(pipeline.bind_group.as_ref().unwrap().0)
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
                self.raw().update_descriptor_sets(&writes, &[]);
            }
            Ok(())
        } else {
            Err(DeviceError::InvalidPipelineHandle)
        }
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.raw().device_wait_idle()?;
        }
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.wait_idle()?;
        self.surface_data =
            self.surface
                .get_data(self.p_device, Resolution { width, height }, false)?;
        Ok(())
    }

    pub fn max_bindless_descriptor_count(&self) -> u32 {
        max_bindless_descriptor_count_inner(&self.p_device_properties)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.wait_idle().ok();

            for mut pipeline in self.pipelines.drain() {
                pipeline.destroy(&self.device);
            }
            for mut shader in self.shaders.drain() {
                shader.destroy(&self.device);
            }

            self.bind_group_pool.destroy(&self.device);

            self.device
                .destroy_pipeline_cache(self.pipeline_cache, None);

            self.device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device
                .destroy_fence(self.setup_commands_reuse_fence, None);
            self.device
                .destroy_fence(self.draw_commands_reuse_fence, None);

            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
        }
    }
}
