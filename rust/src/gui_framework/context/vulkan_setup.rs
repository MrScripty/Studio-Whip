use ash::vk;
use ash::Entry;
use ash::ext::debug_utils;
use ash_window;
use std::ffi::{c_void, CStr};
use std::marker::PhantomData;
use std::sync::Arc;
use vk_mem::Allocator;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::{error, warn, info};

// --- Debug Callback Function ---
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;
    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("?")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };
    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("?")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let log_prefix = "[Vulkan Validation]";

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            // info!("{} VERBOSE [{}({})]:\n{}", log_prefix, message_id_name, message_id_number, message); // Too noisy usually
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("{} INFO [{}({})]:\n{}", log_prefix, message_id_name, message_id_number, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("{} WARNING [{}({})]:\n{}", log_prefix, message_id_name, message_id_number, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("{} ERROR [{}({})]:\n{}", log_prefix, message_id_name, message_id_number, message);
        }
        _ => { // Treat unknown flags as errors
             error!("{} UNKNOWN SEVERITY [{}({})]:\n{}", log_prefix, message_id_name, message_id_number, message);
        }
    }

    vk::FALSE // Should return false unless testing the layers themselves
}

pub fn setup_vulkan(app: &mut VulkanContext, window: &winit::window::Window) {
    // Get handles directly from the window reference
    let display_handle = window.display_handle()
        .expect("Failed to get display handle from winit window")
        .as_raw(); // Get the RawDisplayHandle
    let window_handle = window.window_handle()
        .expect("Failed to get window handle from winit window")
        .as_raw(); // Get the RawWindowHandle

    println!("[setup_vulkan] Loading Vulkan entry...");
    let entry = unsafe { Entry::load() }.expect("Failed to load Vulkan entry");
    app.entry = Some(entry.clone());
    println!("[setup_vulkan] Vulkan entry loaded.");

    println!("[setup_vulkan] Enumerating required surface extensions...");
    let mut surface_extensions = ash_window::enumerate_required_extensions(display_handle)
        .expect("Failed to enumerate required surface extensions")
        .to_vec(); // Convert to Vec to add more extensions

    // Add Debug Utils extension if validation layers are enabled
    #[cfg(debug_assertions)]
    {
        surface_extensions.push(debug_utils::NAME.as_ptr());
        println!("[setup_vulkan] Added Debug Utils extension.");
    }
    println!("[setup_vulkan] Required surface extensions enumerated.");

    // TODO: Add validation layer setup here if desired
    //let layers = []; // Use this if no validation required
    // Enable validation layers in debug builds
    #[cfg(debug_assertions)]
    let layers = unsafe {
        [std::ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()]
    };
    #[cfg(not(debug_assertions))]
    let layers = [];
    #[cfg(debug_assertions)]
    println!("[setup_vulkan] Enabling Validation Layers.");

    let instance_desc = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        // p_application_info: &app_info, // Optional: Add application info
        enabled_layer_count: layers.len() as u32,
        pp_enabled_layer_names: layers.as_ptr(),
        enabled_extension_count: surface_extensions.len() as u32,
        pp_enabled_extension_names: surface_extensions.as_ptr(),
        ..Default::default()
    };
    println!("[setup_vulkan] Creating Vulkan instance...");
    let instance = unsafe { entry.create_instance(&instance_desc, None) }
        .expect("Failed to create Vulkan instance");
    app.instance = Some(instance.clone());
    println!("[setup_vulkan] Vulkan instance created.");

    // --- Create Debug Messenger (after instance, before device) ---
    #[cfg(debug_assertions)]
    {
        let _debug_info = vk::DebugUtilsMessengerCreateInfoEXT {
            s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                ,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(vulkan_debug_callback),
            p_user_data: std::ptr::null_mut(), // No user data needed for this callback
            _marker: PhantomData, // Add the marker field
        };
    }


    println!("[setup_vulkan] Creating surface loader...");
    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
    app.surface_loader = Some(surface_loader.clone());
    println!("[setup_vulkan] Surface loader created.");

    println!("[setup_vulkan] Creating Vulkan surface...");
    let surface = unsafe {
        ash_window::create_surface(
            &entry,
            &instance,
            display_handle,
            window_handle,
            None,
        )
    }
    .expect("Failed to create Vulkan surface");
    app.surface = Some(surface);
    println!("[setup_vulkan] Vulkan surface created.");

    println!("[setup_vulkan] Selecting physical device and queue family...");
    // Find suitable physical device and queue family index
    let (physical_device, queue_family_index) = unsafe {
        instance.enumerate_physical_devices()
            .expect("Failed to enumerate physical devices")
    }
    .into_iter()
    .find_map(|pd| {
        let props = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        props.iter().enumerate().find_map(|(index, qf)| {
            let supports_graphics = qf.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let supports_surface = unsafe {
                surface_loader
                    .get_physical_device_surface_support(pd, index as u32, surface)
                    .unwrap_or(false)
            };
            if supports_graphics && supports_surface {
                Some((pd, index as u32)) // Return physical device and queue index
            } else {
                None
            }
        })
    })
    .expect("Failed to find suitable GPU and queue family");

    // Store the found queue family index and physical device
    app.physical_device = Some(physical_device); // Store the physical device
    app.queue_family_index = Some(queue_family_index);
    println!("[setup_vulkan] Selected queue family index: {}", queue_family_index);

    // Print selected GPU name
    let gpu_properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let gpu_name = unsafe { CStr::from_ptr(gpu_properties.device_name.as_ptr()) }.to_str().unwrap_or("Unknown GPU");
    println!("[setup_vulkan] Selected GPU: {}", gpu_name); // Use log instead of direct print

    println!("[setup_vulkan] Creating logical device and queue...");
    // Create logical device and queue
    let (device, queue) = {
        let queue_priority = 1.0f32;
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: &queue_priority,
            ..Default::default()
        };
        // Required device extensions (Swapchain is essential)
        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        // Optional features (can be queried from physical device)
        let features = vk::PhysicalDeviceFeatures {
            // Enable features needed later, e.g., samplerAnisotropy
            ..Default::default()
        };
        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_create_info,
            enabled_extension_count: device_extensions.len() as u32,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: &features,
            ..Default::default()
        };
        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }
            .expect("Failed to create logical device");
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) }; // Get queue 0 from the family
        (device, queue)
    };
    app.device = Some(device.clone());
    app.queue = Some(queue);
    println!("[setup_vulkan] Logical device and queue created.");

    println!("[setup_vulkan] Creating vk-mem allocator...");
    // Create vk-mem allocator
    let allocator = Arc::new(unsafe {
        Allocator::new(vk_mem::AllocatorCreateInfo::new(
            &instance,
            &device,
            physical_device,
        ))
    }
    .expect("Failed to create vk-mem allocator"));
    app.allocator = Some(allocator);
    println!("[setup_vulkan] vk-mem allocator created.");
    println!("[setup_vulkan] Setup complete.");
}

pub fn cleanup_vulkan(app: &mut VulkanContext) {
    println!("[cleanup_vulkan] Starting cleanup...");
    // Ensure device is idle before destroying anything critical
    if let Some(device) = app.device.as_ref() {
        println!("[cleanup_vulkan] Waiting for device idle...");
        unsafe { device.device_wait_idle().expect("Failed to wait for device idle"); }
        println!("[cleanup_vulkan] Device idle.");
    } else {
        println!("[cleanup_vulkan] Warning: Device not available for idle wait.");
        // Cannot proceed safely if device doesn't exist
        return;
    }

    // Drop the allocator Arc. The allocator itself will be destroyed when the last Arc is dropped.
    // This MUST happen before destroying the device.
    println!("[cleanup_vulkan] Preparing to drop Allocator Arc...");
    if let Some(allocator_arc) = app.allocator.take() {
        drop(allocator_arc); // Explicitly drop the Arc held by VulkanContext
        println!("[cleanup_vulkan] Allocator Arc dropped (vmaDestroyAllocator called by Drop impl).");
    } else {
         println!("[cleanup_vulkan] Allocator already taken or never initialized.");
    }

    // Destroy the logical device
    if let Some(device) = app.device.take() {
        println!("[cleanup_vulkan] Destroying logical device...");
        unsafe { device.destroy_device(None); }
        println!("[cleanup_vulkan] Logical device destroyed.");
    } else {
        println!("[cleanup_vulkan] Device already taken or never initialized.");
    }

    // Destroy the surface
    if let (Some(_instance), Some(surface_loader), Some(surface)) = (app.instance.as_ref(), app.surface_loader.as_ref(), app.surface.take()) {
         println!("[cleanup_vulkan] Destroying surface...");
         unsafe { surface_loader.destroy_surface(surface, None); }
         println!("[cleanup_vulkan] Surface destroyed.");
    } else {
         println!("[cleanup_vulkan] Instance, surface loader, or surface not available for surface destruction.");
    }

    // Destroy Debug Messenger *before* instance
    #[cfg(debug_assertions)]
    if let (Some(loader), Some(messenger)) = (app.debug_utils_loader.take(), app.debug_messenger.take()) {
        println!("[cleanup_vulkan] Destroying debug messenger...");
        unsafe { loader.destroy_debug_utils_messenger(messenger, None); }
        println!("[cleanup_vulkan] Debug messenger destroyed.");
    } else {
         println!("[cleanup_vulkan] Debug messenger loader or handle not available for destruction.");
    }


    // Destroy the instance
    if let Some(instance) = app.instance.take() {
        println!("[cleanup_vulkan] Destroying instance...");
        unsafe { instance.destroy_instance(None); }
        println!("[cleanup_vulkan] Instance destroyed.");
    } else {
         println!("[cleanup_vulkan] Instance already taken or never initialized.");
    }

    // Clear other Option fields just in case
    app.entry = None;
    app.surface_loader = None;
    app.queue = None;
    app.queue_family_index = None;
    // Swapchain resources should be cleaned up by Renderer::cleanup

    println!("[cleanup_vulkan] Cleanup finished.");
}