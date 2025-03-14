use ash::vk;
use ash::{Entry};
use ash_window;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::sync::Arc;
use vk_mem::Allocator;
use winit::window::Window;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use crate::vulkan_context::Platform;

pub fn setup_vulkan(app: &mut Platform, window: Arc<Window>) {
    let entry = unsafe { Entry::load() }.unwrap();
    app.entry = Some(entry.clone());

    let surface_extensions = ash_window::enumerate_required_extensions(
        window.display_handle().unwrap().as_raw(),
    )
    .unwrap();
    let instance_desc = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::InstanceCreateFlags::empty(),
        p_application_info: std::ptr::null(),
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: surface_extensions.len() as u32,
        pp_enabled_extension_names: surface_extensions.as_ptr(),
        _marker: PhantomData,
    };
    let instance = unsafe { entry.create_instance(&instance_desc, None) }.unwrap();
    app.instance = Some(instance.clone());

    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
    app.surface_loader = Some(surface_loader.clone());

    let surface = unsafe {
        ash_window::create_surface(
            &entry,
            &instance,
            window.display_handle().unwrap().as_raw(),
            window.window_handle().unwrap().as_raw(),
            None,
        )
    }
    .unwrap();
    app.surface = Some(surface);

    let (physical_device, queue_family_index) = unsafe {
        instance.enumerate_physical_devices().unwrap()
    }
    .into_iter()
    .find_map(|pd| {
        let props = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        props.iter().position(|qf| {
            qf.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                && unsafe {
                    surface_loader
                        .get_physical_device_surface_support(pd, 0, surface)
                        .unwrap_or(false)
                }
        })
        .map(|index| (pd, index as u32))
    })
    .unwrap();

    println!(
        "Selected GPU: {}",
        unsafe {
            CStr::from_ptr(
                instance
                    .get_physical_device_properties(physical_device)
                    .device_name
                    .as_ptr(),
            )
        }
        .to_str()
        .unwrap()
    );

    let (device, queue) = {
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index,
            queue_count: 1,
            p_queue_priorities: [1.0].as_ptr(),
            _marker: PhantomData,
        };
        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_create_info,
            enabled_extension_count: 1,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: std::ptr::null(),
            ..Default::default() // Use default to handle deprecated fields
        };
        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }.unwrap();
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        (device, queue)
    };
    app.device = Some(device.clone());
    app.queue = Some(queue);

    let allocator = Arc::new(unsafe {
        Allocator::new(vk_mem::AllocatorCreateInfo::new(
            &instance,
            &device,
            physical_device,
        ))
    }
    .unwrap());
    app.allocator = Some(allocator);
}

pub fn cleanup_vulkan(app: &mut Platform) {
    let device = app.device.take().unwrap();
    let surface_loader = app.surface_loader.take().unwrap();
    let allocator = app.allocator.take().unwrap();

    unsafe {
        device.device_wait_idle().unwrap();
        drop(allocator); // Explicitly drop allocator before device
        device.destroy_device(None);
        surface_loader.destroy_surface(app.surface.take().unwrap(), None);
        app.instance.take().unwrap().destroy_instance(None);
    }
}