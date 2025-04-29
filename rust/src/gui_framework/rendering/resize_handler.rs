use ash::vk;
use bevy_math::Mat4;
use bevy_log::{info, warn, error};
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers, cleanup_swapchain_resources}; // Import new functions

pub struct ResizeHandler;

impl ResizeHandler {
    // Only handles swapchain/framebuffer recreation.
    pub fn resize(
        vulkan_context: &mut VulkanContext,
        logical_extent: vk::Extent2D,
        // Removed: uniform_allocation: &mut vk_mem::Allocation,
    ) {
        // Get device early for wait_idle
        let device = vulkan_context.device.as_ref().expect("Device not available for resize").clone(); // Clone device handle

        // Wait for device to be idle before destroying/recreating resources
        unsafe { device.device_wait_idle().unwrap(); }

        // --- Perform operations requiring mutable access to vulkan_context ---

        // 1. Cleanup old swapchain resources (Framebuffers, ImageViews, RenderPass, Swapchain)
        cleanup_swapchain_resources(vulkan_context);

        // 2. Recreate swapchain with the new extent, get actual chosen extent back
        let surface_format = create_swapchain(vulkan_context, logical_extent);

        // 3. Recreate framebuffers uses the extent stored in vulkan_context
        create_framebuffers(vulkan_context, surface_format);
    }
}