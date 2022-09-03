use super::AsRendererContext;
use ash::vk;
use std::{
    borrow::Cow,
    ffi::{CStr, CString},
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
    #[error("Failed to create instance {0}")]
    InstanceCreationFailed(ash::vk::Result),
    #[error("Failed to create debug utils {0}")]
    DebugUtilsCreationFailed(ash::vk::Result),
}

pub struct RendererContext {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl AsRendererContext for RendererContext {
    type CreateError = RendererContextInitError;

    fn create() -> Result<Self, Self::CreateError> {
        let entry = unsafe { ash::Entry::load()? };

        // TODO: Configurable layers
        let layers = layer_names();
        let layers: Vec<*const i8> = layers.iter().map(|raw_name| raw_name.as_ptr()).collect();

        // TODO: COnfigurable
        // TODO: Need to chain with required extensions from window handle
        let extensions = extensions();
        let extensions: Vec<*const i8> = extensions
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

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

        Ok(RendererContext {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
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
