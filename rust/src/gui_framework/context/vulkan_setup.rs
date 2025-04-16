use ash::vk;
use ash::Entry;
use ash_window;
use std::ffi::CStr;
use std::sync::Arc;
use vk_mem::Allocator;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use crate::gui_framework::context::vulkan_context::VulkanContext;

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
    let surface_extensions = ash_window::enumerate_required_extensions(display_handle)
    .expect("Failed to enumerate required surface extensions");
    println!("[setup_vulkan] Required surface extensions enumerated.");

    // TODO: Add validation layer setup here if desired
    let layers = []; // No layers for now
    // let layers = [c"VK_LAYER_KHRONOS_validation".as_ptr()]; // Example for validation

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
    .expect("Failed to find suitable GPU and queue family"); // Add expect

    // Store the found queue family index
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
    if let Some(allocator_arc) = app.allocator.take() {
        drop(allocator_arc); // Explicitly drop the Arc held by VulkanContext
        println!("[cleanup_vulkan] Allocator Arc dropped.");
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