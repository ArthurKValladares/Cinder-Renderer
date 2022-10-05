use crate::{
    context::{
        render_context::{RenderContext, RenderContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
    },
    debug::vulkan_debug_callback,
    resoruces::{
        bind_group::{BindGroupAllocator, BindGroupLayoutCache},
        buffer::{Buffer, BufferDescription},
        memory::Memory,
        pipeline::{GraphicsPipeline, GraphicsPipelineDescription},
        render_pass::{RenderPass, RenderPassDescription},
        sampler::Sampler,
        shader::{Shader, ShaderDescription},
        texture::{self, ImageCreateError, Texture, TextureDescription},
    },
    surface::{Surface, SurfaceData},
    swapchain::Swapchain,
    util::find_memory_type_index,
    InitData,
};
use anyhow::Result;
use ash::vk;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::{
    KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn, KhrPortabilitySubsetFn,
};
use math::{rect::Rect2D, size::Size2D};
use raw_window_handle::HasRawDisplayHandle;
use std::{
    ffi::{CStr, CString},
    ops::Deref,
    os::raw::c_char,
};
use thiserror::Error;
use tracing::{span, Level};

fn submit_work(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
) -> Result<()> {
    let command_buffers = vec![command_buffer];

    let submit_info = vk::SubmitInfo::builder()
        .wait_semaphores(wait_semaphores)
        .wait_dst_stage_mask(wait_mask)
        .command_buffers(&command_buffers)
        .signal_semaphores(signal_semaphores)
        .build();

    unsafe { device.queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence) }?;
    Ok(())
}

// TODO: Get this from the shader later on
#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

#[derive(Debug, Error)]
pub enum DeviceInitError {
    #[error("No suitable device found")]
    NoSuitableDevice,
    #[error(transparent)]
    ImageCreateError(#[from] ImageCreateError),
}

#[derive(Debug, Error)]
pub enum BufferCreateError {
    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}

// TODO: This is rough for now, will be configurable later
fn layer_names() -> Vec<CString> {
    let mut layers = Vec::new();
    layers.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
    layers
}

fn extensions() -> Vec<&'static CStr> {
    let mut extensions = Vec::new();
    extensions.push(ash::extensions::ext::DebugUtils::name());
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        extensions.push(KhrPortabilityEnumerationFn::name());
        // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
        extensions.push(KhrGetPhysicalDeviceProperties2Fn::name());
    }
    extensions
}

// TODO: definitely need a depth image, do it very soon
pub struct Device {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,

    p_device: vk::PhysicalDevice,
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,

    surface: Surface,
    swapchain: Swapchain,

    surface_data: SurfaceData,

    pub depth_image: Texture,
    command_pool: vk::CommandPool,

    pub bind_group_alloc: BindGroupAllocator,
    pub bind_group_cache: BindGroupLayoutCache,

    // TODO: Probably will have better syncronization in the future
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,
}

impl Device {
    pub fn new(window: &winit::window::Window, init_data: InitData) -> Result<Self> {
        let span = span!(Level::DEBUG, "Device::new");
        let _enter = span.enter();

        let entry = unsafe { ash::Entry::load()? };

        // TODO: Configurable layers
        let layers = layer_names();
        let layers = layers
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect::<Vec<*const c_char>>();

        // TODO: Configurable
        let extensions = extensions();
        let extensions = {
            let window_extensions =
                ash_window::enumerate_required_extensions(window.raw_display_handle())?;
            let mut extensions = extensions
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect::<Vec<*const c_char>>();
            extensions.extend(window_extensions.iter());
            extensions
        };

        // TODO: Configurable
        let app_info = vk::ApplicationInfo::builder().api_version(vk::make_api_version(0, 1, 3, 0));
        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };
        let instance_ci = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .flags(create_flags);

        let instance = unsafe { entry.create_instance(&instance_ci, None)? };

        let debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);
        // TODO: Configurable
        let debug_utils_messenger_ci = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));
        let debug_utils_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&debug_utils_messenger_ci, None)? };

        let surface = Surface::new(window, &entry, &instance)?;

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
            .max_by_key(
                |(device, queue_family_index, properties)| match properties.device_type {
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 200,
                    vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 1,
                    _ => 0,
                },
            )
            .ok_or(DeviceInitError::NoSuitableDevice)?;

        let p_device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(p_device) };

        let device_extension_names = [
            ash::extensions::khr::Swapchain::name(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            KhrPortabilitySubsetFn::name(),
        ];
        let device_extension_names_raw: Vec<*const i8> = device_extension_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let features = vk::PhysicalDeviceFeatures::builder();
        let priorities = [1.0];
        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities)
            .build()];
        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);
        let device = unsafe { instance.create_device(p_device, &device_create_info, None) }?;

        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let surface_data = surface.get_data(p_device, init_data.backbuffer_resolution)?;

        let swapchain = Swapchain::new(&instance, &device, &surface, &surface_data)?;

        let depth_image = Texture::create(
            &device,
            &p_device_memory_properties,
            TextureDescription {
                format: texture::Format::D32SFloat,
                usage: texture::Usage::Depth,
                size: Size2D::new(
                    surface_data.surface_resolution.width,
                    surface_data.surface_resolution.height,
                ),
            },
        )?;

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        let rendering_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;

        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let draw_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;
        let setup_commands_reuse_fence = unsafe { device.create_fence(&fence_create_info, None) }?;

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        let command_pool = unsafe { device.create_command_pool(&pool_create_info, None) }?;

        // TODO: is this the right place for the DescriptorPool
        let descriptor_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1 as u32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
            },
        ];

        let bind_group_alloc = BindGroupAllocator::default();
        let bind_group_cache = BindGroupLayoutCache::default();

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
            p_device,
            p_device_properties,
            p_device_memory_properties,
            device,
            queue_family_index,
            present_queue,
            surface,
            swapchain,
            surface_data,
            depth_image,
            present_complete_semaphore,
            rendering_complete_semaphore,
            draw_commands_reuse_fence,
            setup_commands_reuse_fence,
            command_pool,
            bind_group_alloc,
            bind_group_cache,
        })
    }

    pub fn surface_format(&self) -> vk::Format {
        self.surface_data.surface_format.format
    }

    pub fn create_buffer(&self, desc: BufferDescription) -> Result<Buffer> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(desc.size)
            .usage(desc.usage.into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None) }?;
        let buffer_memory_req = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let buffer_memory_index = find_memory_type_index(
            &buffer_memory_req,
            &self.p_device_memory_properties,
            desc.memory_desc.ty.into(),
        )
        .ok_or_else(|| BufferCreateError::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: buffer_memory_req.size,
            memory_type_index: buffer_memory_index,
            ..Default::default()
        };
        let buffer_memory = unsafe { self.device.allocate_memory(&allocate_info, None) }?;

        let memory = Memory {
            raw: buffer_memory,
            req: buffer_memory_req,
        };

        Ok(Buffer {
            raw: buffer,
            memory,
            size_bytes: desc.size,
        })
    }

    pub fn copy_data_to_buffer<T: Copy>(&self, buffer: &Buffer, data: &[T]) -> Result<()> {
        let ptr = unsafe {
            self.device.map_memory(
                buffer.memory.raw,
                0,
                buffer.memory.req.size,
                vk::MemoryMapFlags::empty(),
            )
        }?;
        {
            let mut slice = unsafe {
                ash::util::Align::new(
                    ptr,
                    std::mem::align_of::<T>() as u64,
                    buffer.memory.req.size,
                )
            };
            slice.copy_from_slice(&data);
        }
        unsafe { self.device.unmap_memory(buffer.memory.raw) };
        Ok(())
    }

    pub fn bind_buffer(&self, buffer: &Buffer) -> Result<()> {
        unsafe {
            self.device
                .bind_buffer_memory(buffer.raw, buffer.memory.raw, 0)
        }?;
        Ok(())
    }

    pub fn bind_texture(&self, texture: &Texture) -> Result<()> {
        unsafe {
            self.device
                .bind_image_memory(texture.raw, texture.memory.raw, 0)
        }?;
        Ok(())
    }

    pub fn create_texture(&self, desc: TextureDescription) -> Result<Texture> {
        Texture::create(&self.device, &self.p_device_memory_properties, desc)
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

        let sampler = unsafe { self.device.create_sampler(&sampler_info, None) }?;

        Ok(Sampler { raw: sampler })
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Result<Shader> {
        Shader::create(&self.device, desc)
    }

    pub fn create_render_pass<const N: usize>(
        &self,
        desc: RenderPassDescription<N>,
    ) -> Result<RenderPass> {
        RenderPass::create(
            &self.device,
            &self.swapchain,
            &self.surface_data,
            &self.depth_image,
            desc,
        )
    }

    pub fn clean_render_pass(&self, render_pass: &mut RenderPass) {
        render_pass.clean(&self.device);
    }

    pub fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescription,
    ) -> Result<GraphicsPipeline> {
        GraphicsPipeline::create(&self.device, &self.surface_data, desc)
    }

    pub fn create_render_context(&self, desc: RenderContextDescription) -> Result<RenderContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(RenderContext::from_command_buffer(command_buffer))
    }

    pub fn create_upload_context(&self, desc: UploadContextDescription) -> Result<UploadContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(UploadContext::from_command_buffer(command_buffer))
    }

    pub fn submit_graphics_work(
        &self,
        context: &RenderContext,
        present_index: u32,
    ) -> Result<bool> {
        submit_work(
            &self.device,
            context.shared.command_buffer,
            self.draw_commands_reuse_fence,
            self.present_queue,
            std::slice::from_ref(&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT),
            std::slice::from_ref(&self.present_complete_semaphore),
            std::slice::from_ref(&self.rendering_complete_semaphore),
        )?;

        let wait_semaphors = [self.rendering_complete_semaphore];
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [present_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphors)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let is_suboptimal = unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(self.present_queue, &present_info)
        }?;
        Ok(is_suboptimal)
    }

    pub fn submit_upload_work(&self, context: &UploadContext) -> Result<()> {
        submit_work(
            &self.device,
            context.shared.command_buffer,
            self.setup_commands_reuse_fence,
            self.present_queue,
            &[],
            &[],
            &[],
        )?;

        Ok(())
    }

    // TODO: probably should totally abstract this from user code
    pub fn acquire_next_image(&self) -> Result<(u32, bool)> {
        let (present_index, is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        }?;
        Ok((present_index, is_suboptimal))
    }

    pub fn surface_size(&self) -> Size2D<u32> {
        Size2D::new(
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
        )
    }

    pub fn surface_rect(&self) -> Rect2D<u32> {
        Rect2D::from_top_right_bottom_left(
            0,
            self.surface_data.surface_resolution.width,
            self.surface_data.surface_resolution.height,
            0,
        )
    }

    pub fn resize(&mut self, backbuffer_resolution: Size2D<u32>) -> Result<()> {
        unsafe {
            self.device.device_wait_idle();

            self.surface_data = self
                .surface
                .get_data(self.p_device, backbuffer_resolution)?;
            self.swapchain.resize(
                &self.instance,
                &self.device,
                &self.surface,
                &self.surface_data,
            )?;
            self.depth_image.clean(&self.device);
            self.depth_image = Texture::create(
                &self.device,
                &self.p_device_memory_properties,
                TextureDescription {
                    format: texture::Format::D32SFloat,
                    usage: texture::Usage::Depth,
                    size: backbuffer_resolution,
                },
            )?;
        }

        Ok(())
    }
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
