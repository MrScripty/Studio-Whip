use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use bevy_log::{info, error}; // Add logging

// Only return format, store chosen extent in platform
pub fn create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR {
    let instance = platform.instance.as_ref().expect("Instance not available for swapchain creation");
    let device = platform.device.as_ref().expect("Device not available for swapchain creation");
    let surface_loader = platform.surface_loader.as_ref().expect("Surface loader not available");
    let surface = platform.surface.expect("Surface not available");
    let queue_family_index = platform.queue_family_index.expect("Queue family index not set");

    // Query surface capabilities and formats using the stored physical device
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

    // Choose surface format (prefer B8G8R8A8_SRGB with SRGB color space)
    let surface_format = surface_formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| surface_formats.first().expect("No surface formats available"))
        .clone();

    // Choose present mode (prefer Mailbox -> FIFO -> Immediate)
    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|&m| m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO); // FIFO is guaranteed available

    // Determine swapchain extent: Use the requested extent, but clamp it to the min/max supported by the surface.
    // Do NOT unconditionally use current_extent just because it's not u32::MAX.
    let swap_extent = vk::Extent2D {
        width: extent.width.clamp(surface_caps.min_image_extent.width, surface_caps.max_image_extent.width),
        height: extent.height.clamp(surface_caps.min_image_extent.height, surface_caps.max_image_extent.height),
    };

    info!("[create_swapchain] Surface Caps: min_extent={:?}, max_extent={:?}, current_extent={:?}",
    surface_caps.min_image_extent, surface_caps.max_image_extent, surface_caps.current_extent);
    info!("[create_swapchain] Requested extent: {:?}, Chosen swap_extent: {:?}", extent, swap_extent);
    // Store the chosen extent in the context
    platform.current_swap_extent = swap_extent;

    // Determine image count (request one more than minimum for smoother rendering)
    let mut image_count = surface_caps.min_image_count + 1;
    if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
        image_count = surface_caps.max_image_count; // Clamp to maximum if defined
    }

    // Create swapchain loader
    let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
    platform.swapchain_loader = Some(swapchain_loader.clone());

    // Create swapchain
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
        old_swapchain: vk::SwapchainKHR::null(), // Handle old swapchain during resize later
        ..Default::default()
    };

    platform.swapchain = Some(unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }
        .expect("Failed to create swapchain"));

    // Get swapchain images
    platform.images = unsafe { swapchain_loader.get_swapchain_images(platform.swapchain.unwrap()) }
        .expect("Failed to get swapchain images");

    // Create image views
    platform.image_views = platform.images.iter().map(|&image| {
        let view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: surface_format.format,
            components: vk::ComponentMapping::default(), // RGBA = default
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

    surface_format
}


// Uses the extent stored in platform
pub fn create_framebuffers(platform: &mut VulkanContext, surface_format: vk::SurfaceFormatKHR) {
    info!("[create_framebuffers] Called with platform.current_swap_extent: {:?}, image_view count: {}", platform.current_swap_extent, platform.image_views.len());
    let device = platform.device.as_ref().expect("Device not available for framebuffer creation");

    // Create Render Pass (if it doesn't exist)
    if platform.render_pass.is_none() {
        let color_attachment = vk::AttachmentDescription {
            format: surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR, // Image layout for presentation
            ..Default::default()
        };
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };
        // Add dependency to ensure render pass waits for image to be available
        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(), // Or COLOR_ATTACHMENT_WRITE if layout transition happens before
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };
        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };
        platform.render_pass = Some(unsafe { device.create_render_pass(&render_pass_info, None) }
            .expect("Failed to create render pass"));
    }

    // Create Framebuffers
    platform.framebuffers = platform.image_views.iter().map(|&view| {
        let attachments = [view];
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
    let swapchain_loader = match platform.swapchain_loader.as_ref() {
         Some(l) => l,
         None => {
             // This might happen if cleanup is called before swapchain was fully created or after full cleanup
             info!("[cleanup_swapchain_resources] Swapchain loader not available, assuming resources are already clean or never created.");
             return;
         }
    };

    info!("[cleanup_swapchain_resources] Cleaning up framebuffers, image views, render pass, and swapchain...");
    unsafe {
        // Destroy Framebuffers
        for fb in platform.framebuffers.drain(..) {
            device.destroy_framebuffer(fb, None);
        }
        info!("[cleanup_swapchain_resources] Framebuffers destroyed.");

        // Destroy Image Views
        for view in platform.image_views.drain(..) {
            device.destroy_image_view(view, None);
        }
        info!("[cleanup_swapchain_resources] Image views destroyed.");
        platform.images.clear(); // Explicitly clear the image handles vector

        // Destroy Render Pass (Only if it exists)
        // Render pass might be shared, only destroy if owned uniquely by swapchain setup?
        // For now, assume it's recreated on resize if needed.
        if let Some(rp) = platform.render_pass.take() {
            device.destroy_render_pass(rp, None);
            info!("[cleanup_swapchain_resources] Render pass destroyed.");
        }

        // Destroy Swapchain (Only if it exists)
        if let Some(sc) = platform.swapchain.take() {
            swapchain_loader.destroy_swapchain(sc, None);
            info!("[cleanup_swapchain_resources] Swapchain destroyed.");
        }
    }
    info!("[cleanup_swapchain_resources] Cleanup complete.");
}