use super::AsRendererContext;
use ash::vk;
use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    os::raw::c_char,
};
use thiserror::Error;

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

pub struct RendererContext {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    p_device: vk::PhysicalDevice,
    queue_family_index: usize,
    p_device_properties: vk::PhysicalDeviceProperties,
}

impl AsRendererContext for RendererContext {
    type CreateError = RendererContextInitError;

    fn create(window: &winit::window::Window) -> Result<Self, Self::CreateError> {
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
                            Some((p_device, index, properties))
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
        Ok(RendererContext {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
            p_device,
            queue_family_index,
            p_device_properties,
        })
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
