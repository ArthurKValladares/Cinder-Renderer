mod debug;

use self::debug::vulkan_debug_callback;
use anyhow::Result;
use ash::vk::{self, KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn};
use raw_window_handle::HasRawDisplayHandle;
use std::{
    ffi::{CStr, CString},
    ops::Deref,
    os::raw::c_char,
};

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

pub struct Instance {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: ash::extensions::ext::DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl Instance {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
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

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
        })
    }

    pub(crate) fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    pub(crate) fn instance(&self) -> &ash::Instance {
        &self.instance
    }
}

impl Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}
