mod command_buffer;

use super::AsRendererContext;
use crate::{context::FrameNumber, init::InitData};
use ash::{vk, Device};
use command_buffer::CommandBufferPool;
use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    os::raw::c_char,
};
use thiserror::Error;

const NUM_COMMAND_BUFFERS: u32 = 3;

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
//

#[derive(Debug, Error)]
pub enum RendererContextInitError {
    #[error(transparent)]
    LoadingError(#[from] ash::LoadingError),
    #[error("Failed to enumerate window extensions {0}")]
    FailedToEnumerateWindowExtensions(ash::vk::Result),
    #[error("Failed to create instance {0}")]
    InstanceCreationFailed(ash::vk::Result),
    #[error("Failed to create debug utils {0}")]
    DebugUtilsCreationFailed(ash::vk::Result),
    #[error("Failed to find a physical device {0}")]
    NoPhysicalDevice(ash::vk::Result),
    #[error("Failed to find a supported physical device")]
    NoSupportedPhysicalDevice,
    #[error("Failed to create surface {0}")]
    FailedToCreateSurface(ash::vk::Result),
}

// TODO: Depth image
pub struct RendererContext {
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

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    command_buffer_pool: CommandBufferPool,
}

impl AsRendererContext for RendererContext {
    type CreateError = RendererContextInitError;

    fn create(
        window: &winit::window::Window,
        init_data: InitData,
    ) -> Result<Self, Self::CreateError> {
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
            let window_extensions = ash_window::enumerate_required_extensions(window)
                .map_err(RendererContextInitError::FailedToEnumerateWindowExtensions)?;
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

        let instance = unsafe {
            entry
                .create_instance(&instance_ci, None)
                .map_err(RendererContextInitError::InstanceCreationFailed)?
        };

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
        let debug_utils_messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&debug_utils_messenger_ci, None)
                .map_err(RendererContextInitError::DebugUtilsCreationFailed)?
        };

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
        let surface = unsafe { ash_window::create_surface(&entry, &instance, window, None) }
            .map_err(RendererContextInitError::FailedToCreateSurface)?;

        let p_devices = unsafe { instance.enumerate_physical_devices() }
            .map_err(RendererContextInitError::NoPhysicalDevice)?;
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
            .ok_or(RendererContextInitError::NoSupportedPhysicalDevice)?;

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
        let device =
            unsafe { instance.create_device(p_device, &device_create_info, None) }.unwrap();

        let present_queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let surface_formats =
            unsafe { surface_loader.get_physical_device_surface_formats(p_device, surface) }
                .unwrap();
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
            unsafe { surface_loader.get_physical_device_surface_capabilities(p_device, surface) }
                .unwrap();
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
            unsafe { surface_loader.get_physical_device_surface_present_modes(p_device, surface) }
                .unwrap();
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
        let swapchain =
            unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();

        let present_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let present_image_views: Vec<vk::ImageView> = present_images
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
                unsafe { device.create_image_view(&create_view_info, None) }.unwrap()
            })
            .collect();

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
            unsafe { device.create_semaphore(&semaphore_create_info, None) }.unwrap();
        let rendering_complete_semaphore =
            unsafe { device.create_semaphore(&semaphore_create_info, None) }.unwrap();

        let command_buffer_pool =
            CommandBufferPool::new(&device, queue_family_index, NUM_COMMAND_BUFFERS);

        Ok(RendererContext {
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
            command_buffer_pool,
        })
    }

    fn submit_frame(&mut self, frame_number: FrameNumber) {
        let (present_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();

        self.record_submit_commandbuffer(
            frame_number,
            self.present_queue,
            &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
            &[self.present_complete_semaphore],
            &[self.rendering_complete_semaphore],
            |device, command_buffer| {},
        );

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[self.rendering_complete_semaphore])
            .swapchains(&[self.swapchain])
            .image_indices(&[present_index])
            .build();
        unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
        }
        .unwrap();
    }
}

impl RendererContext {
    pub fn record_submit_commandbuffer<F: FnOnce(&Device, vk::CommandBuffer)>(
        &self,
        frame_number: FrameNumber,
        submit_queue: vk::Queue,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        f: F,
    ) {
        let command_buffer = self.command_buffer_pool.get_command_buffer(frame_number);

        unsafe {
            self.device
                .wait_for_fences(&[command_buffer.fence()], true, std::u64::MAX)
                .unwrap();
            self.device.reset_fences(&[command_buffer.fence()]).unwrap();
        }

        command_buffer.reset(&self.device);
        command_buffer.begin(&self.device);
        f(&self.device, command_buffer.raw());
        command_buffer.end(&self.device);

        let command_buffers = vec![command_buffer.raw()];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&[command_buffer.raw()])
            .signal_semaphores(signal_semaphores)
            .build();
        unsafe {
            self.device
                .queue_submit(submit_queue, &[submit_info], command_buffer.fence())
        }
        .expect("queue submit failed.");
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}
