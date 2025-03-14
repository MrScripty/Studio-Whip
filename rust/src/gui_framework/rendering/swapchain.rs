use ash::vk;
use ash::khr::swapchain;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use std::marker::PhantomData;

pub fn create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR {
    let instance = platform.instance.as_ref().unwrap();
    let device = platform.device.as_ref().unwrap();
    let surface = platform.surface.unwrap();
    let surface_loader = platform.surface_loader.as_ref().unwrap();
    let physical_device = unsafe { instance.enumerate_physical_devices().unwrap() }[0];

    let swapchain_loader = swapchain::Device::new(instance, device);
    platform.swapchain_loader = Some(swapchain_loader.clone());

    let surface_formats = unsafe {
        surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()
    };
    let surface_format = surface_formats[0];

    let surface_caps = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap()
    };
    let mut final_extent = extent;
    if surface_caps.current_extent.width != u32::MAX {
        final_extent = surface_caps.current_extent;
    } else {
        final_extent.width = final_extent.width.clamp(
            surface_caps.min_image_extent.width,
            surface_caps.max_image_extent.width,
        );
        final_extent.height = final_extent.height.clamp(
            surface_caps.min_image_extent.height,
            surface_caps.max_image_extent.height,
        );
    }

    let (swapchain, images) = {
        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: 2,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: final_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            _marker: PhantomData,
        };
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
        (swapchain, images)
    };
    platform.swapchain = Some(swapchain);
    platform.images = images;

    platform.image_views = platform
        .images
        .iter()
        .map(|&image| {
            let view_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
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
                _marker: PhantomData,
            };
            unsafe { device.create_image_view(&view_create_info, None) }.unwrap()
        })
        .collect();

    surface_format
}

pub fn create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) {
    let device = platform.device.as_ref().unwrap();
    let render_pass = {
        let attachment = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format: surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        };
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
            _marker: PhantomData,
        };
        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: 1,
            p_attachments: &attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
            _marker: PhantomData,
        };
        unsafe { device.create_render_pass(&render_pass_info, None) }.unwrap()
    };
    platform.render_pass = Some(render_pass);

    platform.framebuffers = platform
        .image_views
        .iter()
        .map(|&view| {
            let framebuffer_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass,
                attachment_count: 1,
                p_attachments: &view,
                width: extent.width,
                height: extent.height,
                layers: 1,
                _marker: PhantomData,
            };
            unsafe { device.create_framebuffer(&framebuffer_info, None) }.unwrap()
        })
        .collect();
}