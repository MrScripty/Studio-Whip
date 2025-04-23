use ash::vk;
use bevy_math::Mat4;
use bevy_log::{info, warn, error};
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers, cleanup_swapchain_resources}; // Import new functions

pub struct ResizeHandler;

impl ResizeHandler {
    pub fn resize(
        vulkan_context: &mut VulkanContext,
        new_extent: vk::Extent2D,
        uniform_allocation: &mut vk_mem::Allocation,
    ) {
        info!("[ResizeHandler::resize] Called (ECS Migration)");
        // Get device early for wait_idle
        let device = vulkan_context.device.as_ref().expect("Device not available for resize").clone(); // Clone device handle

        // Wait for device to be idle before destroying/recreating resources
        // Use the cloned handle for safety, although &mut vulkan_context implies exclusive access
        unsafe { device.device_wait_idle().unwrap(); }
        info!("[ResizeHandler::resize] Device idle.");

        // --- Perform operations requiring mutable access to vulkan_context ---

        // 1. Cleanup old swapchain resources (Framebuffers, ImageViews, RenderPass, Swapchain)
        // This takes &mut vulkan_context
        cleanup_swapchain_resources(vulkan_context);

        // 2. Recreate swapchain with the new extent, get actual chosen extent back
        // This takes &mut vulkan_context
        let surface_format = create_swapchain(vulkan_context, new_extent);
        info!("[ResizeHandler::resize] Swapchain recreated, actual extent stored: {:?}", vulkan_context.current_swap_extent);

        // 3. Recreate framebuffers uses the extent stored in vulkan_context
        // This takes &mut vulkan_context
        create_framebuffers(vulkan_context, surface_format);
        info!("[ResizeHandler::resize] Framebuffers recreated.");

        // --- Update Uniform Buffer (Requires allocator, immutable borrow starts here) ---

        // Get allocator reference *after* mutable borrows are done
        let allocator = vulkan_context.allocator.as_ref().expect("Allocator not available for resize");

        // 4. Update projection matrix using the *actual* swap extent stored in the context
        let proj_matrix = Mat4::orthographic_rh(0.0, vulkan_context.current_swap_extent.width as f32, 0.0, vulkan_context.current_swap_extent.height as f32, -100.0, 100.0);
        unsafe {
            // Use get_allocation_info for persistently mapped buffer
            let info = allocator.get_allocation_info(uniform_allocation);
            if !info.mapped_data.is_null() {
                let float_ptr = info.mapped_data.cast::<f32>();
                float_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
                // No need to map/unmap if persistently mapped
                info!("[ResizeHandler::resize] Projection matrix updated (via mapped pointer)");
            } else {
                // Fallback if not persistently mapped (shouldn't happen with current setup)
                warn!("[ResizeHandler::resize] Uniform buffer not persistently mapped, attempting map/unmap.");
                match allocator.map_memory(uniform_allocation) {
                    Ok(data_ptr) => {
                        let float_ptr = data_ptr.cast::<f32>();
                        float_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
                        allocator.unmap_memory(uniform_allocation);
                        info!("[ResizeHandler::resize] Projection matrix updated (via map/unmap)");
                    }
                    Err(e) => {
                        error!("[ResizeHandler::resize] Failed to map uniform buffer for resize update: {:?}", e);
                        // Handle error appropriately, maybe panic or return Result
                    }
                }
            }
        } // Immutable borrow of allocator ends here
    }
}