use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::{info, error, warn};
use vk_mem::{Alloc, AllocationCreateInfo}; // For depth image allocation
use crate::gui_framework::context::vulkan_setup::set_debug_object_name;

// Helper to find supported depth format
fn find_supported_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Option<vk::Format> {
    candidates.iter().cloned().find(|format| {
        let props = unsafe { instance.get_physical_device_format_properties(physical_device, *format) };
        match tiling {
            vk::ImageTiling::LINEAR => props.linear_tiling_features.contains(features),
            vk::ImageTiling::OPTIMAL => props.optimal_tiling_features.contains(features),
            _ => false,
        }
    })
}

// Only return format, store chosen extent in platform
pub fn create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR {
    let instance = platform.instance.as_ref().expect("Instance not available for swapchain creation");
    let device = platform.device.as_ref().expect("Device not available for swapchain creation");
    let surface_loader = platform.surface_loader.as_ref().expect("Surface loader not available");
    let surface = platform.surface.expect("Surface not available");
    let queue_family_index = platform.queue_family_index.expect("Queue family index not set");
    let physical_device = platform.physical_device.expect("Physical device not set in VulkanContext");

    let surface_caps = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
    }.expect("Failed to query surface capabilities");

    let surface_formats = unsafe {
        surface_loader.get_physical_device_surface_formats(physical_device, surface)
    }.expect("Failed to query surface formats");

    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
    }.expect("Failed to query present modes");

    let surface_format = surface_formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| surface_formats.first().expect("No surface formats available"))
        .clone();

    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|&m| m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let swap_extent = vk::Extent2D {
        width: extent.width.clamp(surface_caps.min_image_extent.width, surface_caps.max_image_extent.width),
        height: extent.height.clamp(surface_caps.min_image_extent.height, surface_caps.max_image_extent.height),
    };
    platform.current_swap_extent = swap_extent;

    let mut image_count = surface_caps.min_image_count + 1;
    if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
        image_count = surface_caps.max_image_count;
    }

    let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
    platform.swapchain_loader = Some(swapchain_loader.clone());

    let swapchain_create_info = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
        surface,
        min_image_count: image_count,
        image_format: surface_format.format,
        image_color_space: surface_format.color_space,
        image_extent: swap_extent,
        image_array_layers: 1,
        image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        image_sharing_mode: vk::SharingMode::EXCLUSIVE,
        queue_family_index_count: 1,
        p_queue_family_indices: &queue_family_index,
        pre_transform: surface_caps.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode,
        clipped: vk::TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
        ..Default::default()
    };

    platform.swapchain = Some(unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }
        .expect("Failed to create swapchain"));

    platform.images = unsafe { swapchain_loader.get_swapchain_images(platform.swapchain.unwrap()) }
        .expect("Failed to get swapchain images");

    platform.image_views = platform.images.iter().map(|&image| {
        let view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: surface_format.format,
            components: vk::ComponentMapping::default(),
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        unsafe { device.create_image_view(&view_info, None) }.expect("Failed to create image view")
    }).collect();

    // --- Create Depth Resources ---
    let depth_format = find_supported_format(
        instance,
        physical_device,
        &[
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ],
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    ).expect("Failed to find suitable depth format");
    platform.depth_format = Some(depth_format);

    let (depth_image, depth_image_allocation) = {
        let image_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            format: depth_format,
            extent: vk::Extent3D { width: swap_extent.width, height: swap_extent.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };
        let alloc_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        unsafe {
             platform.allocator.as_ref().unwrap().create_image(&image_info, &alloc_info)
            }.expect("Failed to create depth image")
        };
        // --- NAME Depth Image & Memory ---
        #[cfg(debug_assertions)]
    if let Some(debug_device_ext) = platform.debug_utils_device.as_ref() { // Get Device ext
        let mem_handle = platform.allocator.as_ref().unwrap().get_allocation_info(&depth_image_allocation).device_memory;
        set_debug_object_name(debug_device_ext, depth_image, vk::ObjectType::IMAGE, "DepthImage"); // Pass ext
        set_debug_object_name(debug_device_ext, mem_handle, vk::ObjectType::DEVICE_MEMORY, "DepthImage_Mem"); // Pass ext
    }
        // --- END NAME ---
        platform.depth_image = Some(depth_image);
        platform.depth_image_allocation = Some(depth_image_allocation);
    
    
        let depth_image_view = unsafe {
        let view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image: depth_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: depth_format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        device.create_image_view(&view_info, None)
    }.expect("Failed to create depth image view");
    platform.depth_image_view = Some(depth_image_view);

    surface_format
}


// Uses the extent stored in platform
pub fn create_framebuffers(platform: &mut VulkanContext, surface_format: vk::SurfaceFormatKHR) {
    let device = platform.device.as_ref().expect("Device not available for framebuffer creation");

    // Create Render Pass (if it doesn't exist). Includes depth
    if platform.render_pass.is_none() {
        let color_attachment = vk::AttachmentDescription {
            format: surface_format.format, // From swapchain
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR, // Clear color buffer
            store_op: vk::AttachmentStoreOp::STORE, // Store results
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR, // Ready for presentation
            ..Default::default()
        };

        let depth_attachment = vk::AttachmentDescription {
            format: platform.depth_format.expect("Depth format not set"), // From context
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR, // Clear depth buffer
            store_op: vk::AttachmentStoreOp::DONT_CARE, // Don't need depth after frame
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, // Ready for depth test
            ..Default::default()
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0, // Index 0
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1, // Index 1
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_depth_stencil_attachment: &depth_attachment_ref, // Point to depth attachment
            ..Default::default()
        };

        // Add dependency to ensure render pass waits for image to be available
        // and transitions layouts correctly. Wait for color attachment output stage.
        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let attachments = [color_attachment, depth_attachment];
        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };
        platform.render_pass = Some(unsafe { device.create_render_pass(&render_pass_info, None) }
            .expect("Failed to create render pass"));
    }

    // Create Framebuffers - Includes depth view
    let depth_view = platform.depth_image_view.expect("Depth image view missing for framebuffer creation");
    info!("[Swapchain::create_framebuffers] Created {} framebuffers.", platform.framebuffers.len()); // Log framebuffer count
    platform.framebuffers = platform.image_views.iter().map(|&color_view| {
        let attachments = [color_view, depth_view]; // Color attachment 0, Depth attachment 1
        let framebuffer_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            render_pass: platform.render_pass.unwrap(),
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            width: platform.current_swap_extent.width,
            height: platform.current_swap_extent.height,
            layers: 1,
            ..Default::default()
        };
        unsafe { device.create_framebuffer(&framebuffer_info, None) }.expect("Failed to create framebuffer")
    }).collect();

    // --- Allocate Command Buffers (one per framebuffer/swapchain image) ---
    // Ensure command pool exists
    let command_pool = platform.command_pool.expect("Command pool not available for command buffer allocation");

    // Free old command buffers if they exist
    if !platform.command_buffers.is_empty() {
        unsafe {
            device.free_command_buffers(command_pool, &platform.command_buffers);
        }
        platform.command_buffers.clear();
        info!("[Swapchain::create_framebuffers] Freed old command buffers.");
    }

    // This check is now more critical: if framebuffers weren't created, we MUST NOT allocate 0 command buffers.
    if platform.framebuffers.is_empty() {
        warn!("[Swapchain::create_framebuffers] No framebuffers were created (e.g., image_views might be empty). Skipping command buffer allocation.");
        platform.command_buffers = Vec::new(); // Ensure it's empty
    } else {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: platform.framebuffers.len() as u32, // This should now be > 0
            _marker: std::marker::PhantomData,
        };

        platform.command_buffers = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .expect("Failed to allocate command buffers")
        };
        info!("[Swapchain::create_framebuffers] Allocated {} command buffers.", platform.command_buffers.len());
    }
}


/// Cleans up swapchain-related resources (ImageViews, Framebuffers, RenderPass, Swapchain itself).
/// Assumes device is idle.
pub fn cleanup_swapchain_resources(platform: &mut VulkanContext) {
    let device = match platform.device.as_ref() {
        Some(d) => d,
        None => {
            error!("[cleanup_swapchain_resources] Device not available, cannot cleanup.");
            return;
        }
    };
    // swapchain_loader is only needed for swapchain.destroy_swapchain
    // let swapchain_loader = match platform.swapchain_loader.as_ref() {
    //      Some(l) => l,
    //      None => {
    //          info!("[cleanup_swapchain_resources] Swapchain loader not available, assuming resources are already clean or never created.");
    //          return;
    //      }
    // };

    unsafe {
        if let Some(pool) = platform.command_pool {
            if !platform.command_buffers.is_empty() {
                device.free_command_buffers(pool, &platform.command_buffers);
                platform.command_buffers.clear();
                info!("[cleanup_swapchain_resources] Freed command buffers.");
            }
        } else {
            if !platform.command_buffers.is_empty() {
                warn!("[cleanup_swapchain_resources] Command buffers exist but command pool is None. Buffers not freed.");
                platform.command_buffers.clear();
            }
        }

        info!("[cleanup_swapchain_resources] Destroying {} framebuffers.", platform.framebuffers.len());
        for fb in platform.framebuffers.drain(..) {
            device.destroy_framebuffer(fb, None);
        }

        info!("[cleanup_swapchain_resources] Destroying {} image views.", platform.image_views.len());
        for view in platform.image_views.drain(..) {
            device.destroy_image_view(view, None);
        }
        platform.images.clear();

        // Destroy Depth Buffer Resources
        if let Some(view) = platform.depth_image_view.take() {
            info!("[cleanup_swapchain_resources] Destroying DepthImageView {:?}.", view);
            device.destroy_image_view(view, None);
        } else {
            info!("[cleanup_swapchain_resources] DepthImageView was already None.");
        }

        let depth_image_opt = platform.depth_image.take();
        let depth_alloc_opt = platform.depth_image_allocation.take();

        // Check for mismatched resources *before* moving them into the `if let`.
        if depth_image_opt.is_some() != depth_alloc_opt.is_some() {
            warn!("[cleanup_swapchain_resources] Mismatched DepthImage and Allocation. Image: {:?}, Allocation: {:?}",
                  depth_image_opt.is_some(), depth_alloc_opt.is_some());
        }

        // Now, consume the Options to perform the cleanup.
        if let (Some(image), Some(mut allocation)) = (depth_image_opt, depth_alloc_opt) {
            info!("[cleanup_swapchain_resources] Attempting to destroy DepthImage {:?} with its allocation.", image);
            if let Some(allocator) = platform.allocator.as_ref() {
                // Use the correct vk-mem function to destroy the image and free its memory.
                allocator.destroy_image(image, &mut allocation);
                info!("[cleanup_swapchain_resources] Destroyed DepthImage and its memory allocation.");
            } else {
                error!("[cleanup_swapchain_resources] Allocator not available to destroy depth image!");
            }
        } else {
            info!("[cleanup_swapchain_resources] DepthImage or its allocation was already taken/None.");
        }
        platform.depth_format = None;

        if let Some(rp) = platform.render_pass.take() {
            info!("[cleanup_swapchain_resources] Destroying RenderPass {:?}.", rp);
            device.destroy_render_pass(rp, None);
        } else {
            info!("[cleanup_swapchain_resources] RenderPass was already None.");
        }

        if let Some(sc) = platform.swapchain.take() {
            // Need swapchain_loader for this
            if let Some(loader) = platform.swapchain_loader.as_ref() {
                info!("[cleanup_swapchain_resources] Destroying Swapchain {:?}.", sc);
                loader.destroy_swapchain(sc, None);
            } else {
                error!("[cleanup_swapchain_resources] Swapchain loader not available to destroy swapchain handle {:?}.", sc);
            }
        } else {
            info!("[cleanup_swapchain_resources] Swapchain was already None.");
        }
    }
    info!("[cleanup_swapchain_resources] Finished cleaning up swapchain resources.");
}