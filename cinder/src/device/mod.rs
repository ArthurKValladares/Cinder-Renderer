use crate::{
    context::{
        graphics_context::{GraphicsContext, GraphicsContextDescription},
        upload_context::{UploadContext, UploadContextDescription},
        Context,
    },
    debug::vulkan_debug_callback,
    resoruces::{
        buffer::{Buffer, BufferDescription},
        pipeline::{Pipeline, PipelineDescription},
        render_pass::{RenderPass, RenderPassDescription},
        shader::{Shader, ShaderDescription},
        texture::{Texture, TextureDescription},
    },
    InitData,
};
use anyhow::Result;
use ash::vk;
use std::{
    ffi::{CStr, CString},
    fs::File,
    ops::Deref,
    os::raw::c_char,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceInitError {
    #[error("No suitable device found")]
    NoSuitableDevice,
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
    extensions
}

// TODO: definitely need a depth image, do it very soon
pub struct Device {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,

    p_device: vk::PhysicalDevice,
    p_device_properties: vk::PhysicalDeviceProperties,
    p_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    device: ash::Device,
    queue_family_index: u32,
    present_queue: vk::Queue,

    surface: vk::SurfaceKHR,
    surface_format: vk::SurfaceFormatKHR,
    surface_resolution: vk::Extent2D,

    swapchain: vk::SwapchainKHR,
    present_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    command_pool: vk::CommandPool,

    // TODO: Probably will have better syncronization in the future
    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
}

impl Device {
    pub fn new(window: &winit::window::Window, init_data: InitData) -> Result<Self> {
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
            let window_extensions = ash_window::enumerate_required_extensions(window)?;
            let mut extensions = extensions
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect::<Vec<*const c_char>>();
            extensions.extend(window_extensions.iter());
            extensions
        };

        let app_info = vk::ApplicationInfo::builder().api_version(vk::make_api_version(0, 1, 3, 0)); // TODO: Configure version
        let instance_ci = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);

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

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe { ash_window::create_surface(&entry, &instance, window, None) }?;

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
                                    surface_loader.get_physical_device_surface_support(
                                        p_device,
                                        index as u32,
                                        surface,
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

        let device_extension_names = [ash::extensions::khr::Swapchain::name()];
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

        let surface_formats =
            unsafe { surface_loader.get_physical_device_surface_formats(p_device, surface) }?;

        let surface_format = surface_formats
            .iter()
            .map(|sfmt| match sfmt.format {
                vk::Format::UNDEFINED => vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8_UNORM,
                    color_space: sfmt.color_space,
                },
                _ => *sfmt,
            })
            .next()
            .expect("Unable to find suitable surface format.");
        let surface_capabilities =
            unsafe { surface_loader.get_physical_device_surface_capabilities(p_device, surface) }?;
        let mut desired_image_count = {
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }
            desired_image_count
        };
        let surface_resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: init_data.backbuffer_resolution.width,
                height: init_data.backbuffer_resolution.height,
            },
            _ => surface_capabilities.current_extent,
        };

        let present_modes =
            unsafe { surface_loader.get_physical_device_surface_present_modes(p_device, surface) }?;
        // TODO: vsyc or not vsync option
        let present_mode_preference = if false {
            vec![vk::PresentModeKHR::FIFO_RELAXED, vk::PresentModeKHR::FIFO]
        } else {
            vec![vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE]
        };
        let present_mode = present_mode_preference
            .into_iter()
            .find(|mode| present_modes.contains(mode))
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }?;

        let present_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }?;
        let present_image_views = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                unsafe { device.create_image_view(&create_view_info, None) }
            })
            .collect::<Result<Vec<vk::ImageView>, ash::vk::Result>>()?;

        let depth_image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .extent(vk::Extent3D {
                width: surface_resolution.width,
                height: surface_resolution.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;
        let rendering_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }?;

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        let command_pool = unsafe { device.create_command_pool(&pool_create_info, None) }?;

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
            surface_loader,
            swapchain_loader,
            p_device,
            p_device_properties,
            p_device_memory_properties,
            device,
            queue_family_index,
            present_queue,
            surface,
            surface_format,
            surface_resolution,
            swapchain,
            present_images,
            present_image_views,
            present_complete_semaphore,
            rendering_complete_semaphore,
            command_pool,
        })
    }

    pub fn create_buffer(&self, desc: BufferDescription) -> Buffer {
        Buffer {}
    }

    pub fn create_texture(&self, desc: TextureDescription) -> Texture {
        Texture {}
    }

    pub fn create_shader(&self, desc: ShaderDescription) -> Result<Shader> {
        let mut spv_file = File::open(desc.path)?;
        let code = ash::util::read_spv(&mut spv_file)?;
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe { self.device.create_shader_module(&shader_info, None)? };
        Ok(Shader { module })
    }

    pub fn create_render_pass(&self, desc: RenderPassDescription) -> RenderPass {
        RenderPass {}
    }

    pub fn create_pipeline(&self, desc: PipelineDescription) -> Pipeline {
        Pipeline {}
    }

    pub fn create_graphics_context(
        &self,
        desc: GraphicsContextDescription,
    ) -> Result<GraphicsContext> {
        // TODO: Allocate buffers in bulk, manage handing them out some way
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        }[0];

        Ok(GraphicsContext::from_command_buffer(command_buffer))
    }

    pub fn create_upload_context(&self, desc: UploadContextDescription) -> UploadContext {
        UploadContext {}
    }

    pub fn submit_work(&self, context: &dyn Context) {}
}

impl Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
