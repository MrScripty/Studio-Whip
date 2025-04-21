use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
// Removed Renderable import
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
// Removed command_buffers import (unused)
// use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use bevy_math::Mat4;
use bevy_log::{info, error};

pub struct ResizeHandler;

impl ResizeHandler {
    // Modified signature: Removed renderables, changed extent type, added global descriptor set
    pub fn resize(
        vulkan_context: &mut VulkanContext,
        new_extent: vk::Extent2D, // Pass the new extent directly
        uniform_allocation: &mut vk_mem::Allocation,
    ) {
        info!("[ResizeHandler::resize] Called (ECS Migration)");
        let device = vulkan_context.device.as_ref().unwrap();
        unsafe { device.device_wait_idle().unwrap() };

        // Cleanup old swapchain resources (Remains the same)
        for &framebuffer in &vulkan_context.framebuffers { unsafe { device.destroy_framebuffer(framebuffer, None) }; }
        if let Some(rp) = vulkan_context.render_pass.take() { unsafe { device.destroy_render_pass(rp, None) }; } else { info!("[ResizeHandler] Render pass already taken/cleaned?"); }
        for &view in &vulkan_context.image_views { unsafe { device.destroy_image_view(view, None) }; }
        vulkan_context.image_views.clear();
        vulkan_context.framebuffers.clear();
        if let Some(swapchain) = vulkan_context.swapchain.take() {
            if let Some(loader) = vulkan_context.swapchain_loader.as_ref() {
                unsafe { loader.destroy_swapchain(swapchain, None) };
            } else { info!("[ResizeHandler] Swapchain loader not available for swapchain destruction?"); }
        } else { info!("[ResizeHandler] Swapchain already taken/cleaned?"); }


        // Create new swapchain resources using new_extent
        let surface_format = create_swapchain(vulkan_context, new_extent);
        create_framebuffers(vulkan_context, new_extent, surface_format);
        info!("[ResizeHandler::resize] New extent: {:?}", new_extent);

        // Update projection matrix in uniform buffer
        let ortho = Mat4::orthographic_rh(0.0, new_extent.width as f32, new_extent.height as f32, 0.0, -1.0, 1.0);
        if let Some(allocator) = vulkan_context.allocator.as_ref() { // Check if allocator exists
            match unsafe { allocator.map_memory(uniform_allocation) } {
                Ok(data_ptr) => {
                    let mapped_ptr = data_ptr.cast::<f32>();
                    unsafe { mapped_ptr.copy_from_nonoverlapping(ortho.to_cols_array().as_ptr(), 16) };
                    unsafe { allocator.unmap_memory(uniform_allocation) };
                    info!("[ResizeHandler::resize] Projection matrix updated");
                }
                Err(e) => {
                    // Log error instead of panicking
                    error!("[ResizeHandler::resize] Failed to map uniform buffer memory during resize: {:?}", e);
                    // Continue without updating the projection if mapping fails
                }
            }
        } else {
            error!("[ResizeHandler::resize] Allocator not available for updating projection matrix.");
        }
    }
}