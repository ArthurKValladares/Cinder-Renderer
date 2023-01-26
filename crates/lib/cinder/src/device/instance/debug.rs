use ash::vk;
use rkyv::de;
use std::{borrow::Cow, ffi::CStr};

pub unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

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

pub fn set_object_name(
    debug_utils: &ash::extensions::ext::DebugUtils,
    device: ash::vk::Device,
    object_type: vk::ObjectType,
    object: impl vk::Handle,
    name: &str,
) {
    let mut buffer: [u8; 64] = [0u8; 64];
    let buffer_vec: Vec<u8>;

    let name_bytes = if name.len() < buffer.len() {
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        buffer[name.len()] = 0;
        &buffer[..name.len() + 1]
    } else {
        buffer_vec = name
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect();
        &buffer_vec
    };

    let name = unsafe { CStr::from_bytes_with_nul_unchecked(name_bytes) };

    let _result = unsafe {
        debug_utils.set_debug_utils_object_name(
            device,
            &vk::DebugUtilsObjectNameInfoEXT::builder()
                .object_type(object_type)
                .object_handle(object.as_raw())
                .object_name(name),
        )
    };
}

pub fn cmd_begin_label(
    debug_utils: &ash::extensions::ext::DebugUtils,
    command_buffer: vk::CommandBuffer,
    name: &str,
    color: [f32; 4],
) {
    let mut buffer: [u8; 64] = [0u8; 64];
    let buffer_vec: Vec<u8>;

    let name_bytes = if name.len() < buffer.len() {
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        buffer[name.len()] = 0;
        &buffer[..name.len() + 1]
    } else {
        buffer_vec = name
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect();
        &buffer_vec
    };

    let name = unsafe { CStr::from_bytes_with_nul_unchecked(name_bytes) };

    unsafe {
        debug_utils.cmd_begin_debug_utils_label(
            command_buffer,
            &vk::DebugUtilsLabelEXT::builder()
                .label_name(name)
                .color(color)
                .build(),
        )
    }
}

pub fn cmd_end_label(
    debug_utils: &ash::extensions::ext::DebugUtils,
    command_buffer: vk::CommandBuffer,
) {
    unsafe { debug_utils.cmd_end_debug_utils_label(command_buffer) }
}

pub fn cmd_insert_label(
    debug_utils: &ash::extensions::ext::DebugUtils,
    command_buffer: vk::CommandBuffer,
    name: &str,
    color: [f32; 4],
) {
    let mut buffer: [u8; 64] = [0u8; 64];
    let buffer_vec: Vec<u8>;

    let name_bytes = if name.len() < buffer.len() {
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        buffer[name.len()] = 0;
        &buffer[..name.len() + 1]
    } else {
        buffer_vec = name
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect();
        &buffer_vec
    };

    let name = unsafe { CStr::from_bytes_with_nul_unchecked(name_bytes) };

    unsafe {
        debug_utils.cmd_insert_debug_utils_label(
            command_buffer,
            &vk::DebugUtilsLabelEXT::builder()
                .label_name(name)
                .color(color)
                .build(),
        )
    }
}

pub fn queue_begin_label(
    debug_utils: &ash::extensions::ext::DebugUtils,
    queue: vk::Queue,
    name: &str,
    color: [f32; 4],
) {
    let mut buffer: [u8; 64] = [0u8; 64];
    let buffer_vec: Vec<u8>;

    let name_bytes = if name.len() < buffer.len() {
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        buffer[name.len()] = 0;
        &buffer[..name.len() + 1]
    } else {
        buffer_vec = name
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect();
        &buffer_vec
    };

    let name = unsafe { CStr::from_bytes_with_nul_unchecked(name_bytes) };

    unsafe {
        debug_utils.queue_begin_debug_utils_label(
            queue,
            &vk::DebugUtilsLabelEXT::builder()
                .label_name(name)
                .color(color)
                .build(),
        )
    }
}

pub fn queue_end_label(debug_utils: &ash::extensions::ext::DebugUtils, queue: vk::Queue) {
    unsafe { debug_utils.queue_end_debug_utils_label(queue) }
}

pub fn queue_insert_label(
    debug_utils: &ash::extensions::ext::DebugUtils,
    queue: vk::Queue,
    name: &str,
    color: [f32; 4],
) {
    let mut buffer: [u8; 64] = [0u8; 64];
    let buffer_vec: Vec<u8>;

    let name_bytes = if name.len() < buffer.len() {
        buffer[..name.len()].copy_from_slice(name.as_bytes());
        buffer[name.len()] = 0;
        &buffer[..name.len() + 1]
    } else {
        buffer_vec = name
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect();
        &buffer_vec
    };

    let name = unsafe { CStr::from_bytes_with_nul_unchecked(name_bytes) };

    unsafe {
        debug_utils.queue_insert_debug_utils_label(
            queue,
            &vk::DebugUtilsLabelEXT::builder()
                .label_name(name)
                .color(color)
                .build(),
        )
    }
}
