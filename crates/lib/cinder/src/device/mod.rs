mod extensions;
mod instance;
mod properties;
mod surface;

pub use self::instance::{debug::*, Instance};
use self::{extensions::DeviceExtensions, properties::DeviceProperties, surface::Surface};
pub use self::{instance::Extension, surface::SurfaceData};
use crate::{
    command_queue::CommandQueue,
    profiling::QueryPool,
    resources::{
        bind_group::{BindGroupBindInfo, BindGroupPool, BindGroupWriteData},
        buffer::{Buffer, BufferDescription, BufferUsage},
        image::{Image, ImageDescription, ImageError},
        manager::ResourceManager,
        pipeline::graphics::{GraphicsPipeline, GraphicsPipelineDescription},
        sampler::{Sampler, SamplerDescription},
        shader::{Shader, ShaderDesc},
    },
};
use anyhow::Result;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::KhrPortabilitySubsetFn;
use ash::{extensions::khr::DynamicRendering, vk};
use math::{rect::Rect2D, size::Size2D};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use resource_manager::ResourceId;
use thiserror::Error;
use util::size_of_slice;

pub const MAX_FRAMES_IN_FLIGHT: usize = 3;
pub const MAX_BINDLESS_RESOURCES: u32 = 1024;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("No suitable device found")]
    NoSuitableDevice,
    #[error(transparent)]
    ImageCreateError(#[from] ImageError),
    #[error(transparent)]
    ResourceManagerError(#[from] crate::resources::manager::ResourceManagerError),
    #[error("Resource not in cache")]
    ResourceNotInCache,
}

pub struct Device {
    p_device: vk::PhysicalDevice,
    properties: DeviceProperties,
    device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,
    surface: Surface,
    instance: Instance,
    pub(crate) pipeline_cache: vk::PipelineCache,
    pub(crate) bind_group_pool: BindGroupPool,
    pub(crate) surface_data: SurfaceData,
    extensions: DeviceExtensions,
    render_complete_semaphores: [vk::Semaphore; MAX_FRAMES_IN_FLIGHT],
    image_acquired_semaphore: vk::Semaphore,
    command_buffer_executed_fences: [vk::Fence; MAX_FRAMES_IN_FLIGHT],
    frame_index: usize,
}

impl Device {
    pub fn new<W>(window: &W, window_width: u32, window_height: u32) -> Result<Self>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
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
            .ok_or(DeviceError::NoSuitableDevice)?;

        let properties = DeviceProperties::new(instance.raw(), p_device, p_device_properties);

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
        // TODO: this is currently crashing for me, look into it later
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
            "Device",
        );
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::PHYSICAL_DEVICE,
            p_device,
            "Physical Device",
        );

        // TODO: Review queue stuff
        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::QUEUE,
            present_queue,
            "Present Queue",
        );

        let ci = vk::PipelineCacheCreateInfo::builder().build();
        let pipeline_cache = unsafe { device.create_pipeline_cache(&ci, None)? };
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::PIPELINE_CACHE,
            pipeline_cache,
            "Pipeline Cache",
        );
        let bind_group_pool = BindGroupPool::new(&instance, &device)?;

        let surface_data = surface.get_data(p_device, window_width, window_height, false)?;

        let extensions = DeviceExtensions::new(&instance, &device);

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let render_complete_semaphores = {
            let mut semaphores = [vk::Semaphore::null(); MAX_FRAMES_IN_FLIGHT];
            for idx in 0..MAX_FRAMES_IN_FLIGHT {
                let semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
                instance::debug::set_object_name(
                    instance.debug(),
                    device.handle(),
                    vk::ObjectType::SEMAPHORE,
                    semaphore,
                    &format!("Render Complete Semaphore {idx}"),
                );
                semaphores[idx] = semaphore;
            }
            semaphores
        };

        let image_acquired_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        instance::debug::set_object_name(
            instance.debug(),
            device.handle(),
            vk::ObjectType::SEMAPHORE,
            image_acquired_semaphore,
            "Image Acquired Semaphore",
        );

        let fence_create_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };

        let command_buffer_executed_fences = {
            let mut fences = [vk::Fence::null(); MAX_FRAMES_IN_FLIGHT];
            for idx in 0..MAX_FRAMES_IN_FLIGHT {
                let fence = unsafe { device.create_fence(&fence_create_info, None) }?;
                instance::debug::set_object_name(
                    instance.debug(),
                    device.handle(),
                    vk::ObjectType::FENCE,
                    fence,
                    &format!("Command Buffer Executed Fence {idx}"),
                );
                fences[idx] = fence;
            }
            fences
        };

        Ok(Self {
            instance,
            surface,
            surface_data,
            p_device,
            properties,
            device,
            queue_family_index,
            present_queue,
            pipeline_cache,
            bind_group_pool,
            extensions,
            render_complete_semaphores,
            image_acquired_semaphore,
            command_buffer_executed_fences,
            frame_index: 0,
        })
    }

    pub fn new_frame(&mut self) -> Result<()> {
        let render_complete_fence = self.command_buffer_executed_fence();
        unsafe {
            match self.device.get_fence_status(render_complete_fence) {
                Ok(false) | Err(_) => {
                    self.device
                        .wait_for_fences(&[render_complete_fence], true, std::u64::MAX)?;
                }
                _ => {}
            }

            self.device.reset_fences(&[render_complete_fence])?;
        }

        Ok(())
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

    pub fn properties(&self) -> vk::PhysicalDeviceProperties {
        self.properties.properties()
    }

    pub fn memopry_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.properties.memory_properties()
    }

    pub fn descriptor_indexing_properties(&self) -> vk::PhysicalDeviceDescriptorIndexingProperties {
        self.properties.descriptor_indexing_properties()
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }

    pub fn dynamic_rendering(&self) -> &DynamicRendering {
        self.extensions.dynamic_rendering()
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
        let mut buffer = Buffer::create(self, size, desc)?;
        buffer.num_elements = Some(data.len() as u32);
        buffer.mem_copy(0, data)?;
        Ok(buffer)
    }

    pub fn create_image(&self, size: Size2D<u32>, desc: ImageDescription) -> Result<Image> {
        Image::create(self, size, desc)
    }

    pub fn create_image_with_data(
        &self,
        size: Size2D<u32>,
        bytes: &[u8],
        cmd_queue: &CommandQueue,
        desc: ImageDescription,
    ) -> Result<Image> {
        let image = Image::create(self, size, desc)?;

        let image_buffer = self.create_buffer_with_data(
            bytes,
            BufferDescription {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
        )?;

        let instant_command_list = cmd_queue.get_immediate_command_list(self)?;
        instant_command_list.set_image_memory_barrier(
            self,
            image.raw,
            vk::ImageAspectFlags::COLOR,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            Default::default(),
        );
        instant_command_list.copy_buffer_to_image(self, &image_buffer, &image);
        instant_command_list.set_image_memory_barrier(
            self,
            image.raw,
            vk::ImageAspectFlags::COLOR,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            Default::default(),
        );
        instant_command_list.end(self)?;
        instant_command_list.immediate_submit(self, self.present_queue)?;
        instant_command_list.reset(self)?;

        image_buffer.destroy(self);

        Ok(image)
    }

    pub fn create_shader(&self, bytes: &[u8], desc: ShaderDesc) -> Result<Shader> {
        Shader::create(self, bytes, desc)
    }

    pub fn recreate_shader(
        &self,
        manager: &mut ResourceManager,
        id: ResourceId<Shader>,
        bytes: &[u8],
    ) -> Result<(), DeviceError> {
        let new = manager
            .shaders
            .get(id)
            .map(|old| Shader::create(self, bytes, old.desc).unwrap());

        if let Some(new) = new {
            manager.replace_shader(id, new, self.current_frame_in_flight());
            Ok(())
        } else {
            Err(DeviceError::ResourceNotInCache)
        }
    }

    pub fn create_graphics_pipeline(
        &self,
        vertex_shader: &Shader,
        fragment_shader: Option<&Shader>,
        desc: GraphicsPipelineDescription,
    ) -> Result<GraphicsPipeline> {
        GraphicsPipeline::create(self, vertex_shader, fragment_shader, desc)
    }

    pub fn recreate_graphics_pipeline(
        &self,
        manager: &mut ResourceManager,
        pipeline_handle: ResourceId<GraphicsPipeline>,
        vertex_handle: ResourceId<Shader>,
        fragment_handle: Option<ResourceId<Shader>>,
    ) -> Result<()> {
        manager.recreate_graphics_pipeline(
            self,
            pipeline_handle,
            vertex_handle,
            fragment_handle,
        )?;
        Ok(())
    }

    pub fn create_sampler(&self, desc: SamplerDescription) -> Result<Sampler> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: desc.filter.into(),
            min_filter: desc.filter.into(),
            mipmap_mode: desc.mipmap_mode.into(),
            address_mode_u: desc.address_mode.into(),
            address_mode_v: desc.address_mode.into(),
            address_mode_w: desc.address_mode.into(),
            max_anisotropy: 1.0,
            border_color: desc.border_color.into(),
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };

        let sampler = unsafe { self.raw().create_sampler(&sampler_info, None) }?;

        if let Some(name) = desc.name {
            self.set_name(vk::ObjectType::SAMPLER, sampler, name);
        }

        Ok(Sampler { raw: sampler })
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

    pub fn surface_aspect_ratio(&self) -> f32 {
        self.surface_data.surface_resolution.width as f32
            / self.surface_data.surface_resolution.height as f32
    }

    pub fn write_bind_group(&self, infos: &[BindGroupBindInfo]) -> Result<(), DeviceError> {
        let writes = infos
            .iter()
            .map(|info| {
                let mut write = vk::WriteDescriptorSet::builder()
                    .dst_set(info.group.0)
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
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.raw().device_wait_idle()?;
        }
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.wait_idle()?;
        self.surface_data = self.surface.get_data(self.p_device, width, height, false)?;
        Ok(())
    }

    pub(crate) fn render_complete_semaphore(&self) -> vk::Semaphore {
        self.render_complete_semaphores[self.current_frame_in_flight()]
    }

    pub(crate) fn image_acquired_semaphore(&self) -> vk::Semaphore {
        self.image_acquired_semaphore
    }

    pub(crate) fn command_buffer_executed_fence(&self) -> vk::Fence {
        self.command_buffer_executed_fences[self.current_frame_in_flight()]
    }

    pub fn current_frame_in_flight(&self) -> usize {
        self.frame_index % MAX_FRAMES_IN_FLIGHT
    }

    pub fn bump_frame(&mut self) {
        self.frame_index += 1;
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.wait_idle().ok();

            self.bind_group_pool.destroy(&self.device);

            self.device
                .destroy_pipeline_cache(self.pipeline_cache, None);

            for fence in &self.command_buffer_executed_fences {
                self.device.destroy_fence(*fence, None);
            }

            for semaphore in &self.render_complete_semaphores {
                self.device.destroy_semaphore(*semaphore, None);
            }

            self.device
                .destroy_semaphore(self.image_acquired_semaphore, None);

            // MUST BE DESTROYED LAST!
            self.device.destroy_device(None);
        }
    }
}
