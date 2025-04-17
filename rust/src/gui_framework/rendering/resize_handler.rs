use ash::vk;
// Removed: use crate::Vertex;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use bevy_math::Mat4;
use bevy_log::warn; // Added warn import

pub struct ResizeHandler;

impl ResizeHandler {
    pub fn resize(
        vulkan_context: &mut VulkanContext,
        renderables: &mut Vec<Renderable>, // Still uses old renderables Vec
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
        width: u32,
        height: u32,
        uniform_allocation: &mut vk_mem::Allocation,
    ) {
        println!("[ResizeHandler::resize] Called (ECS Migration)");
        let device = vulkan_context.device.as_ref().unwrap();
        unsafe { device.device_wait_idle().unwrap() };

        // Cleanup old swapchain resources
        for &framebuffer in &vulkan_context.framebuffers { unsafe { device.destroy_framebuffer(framebuffer, None) }; }
        if let Some(rp) = vulkan_context.render_pass.take() { unsafe { device.destroy_render_pass(rp, None) }; } else { println!("[ResizeHandler] Render pass already taken/cleaned?"); }
        for &view in &vulkan_context.image_views { unsafe { device.destroy_image_view(view, None) }; }
        vulkan_context.image_views.clear(); // Clear the vec after destroying views
        vulkan_context.framebuffers.clear(); // Clear the vec after destroying framebuffers
        if let Some(swapchain) = vulkan_context.swapchain.take() {
            if let Some(loader) = vulkan_context.swapchain_loader.as_ref() {
                unsafe { loader.destroy_swapchain(swapchain, None) };
            } else { println!("[ResizeHandler] Swapchain loader not available for swapchain destruction?"); }
        } else { println!("[ResizeHandler] Swapchain already taken/cleaned?"); }


        // Create new swapchain resources
        let extent = vk::Extent2D { width, height };
        let surface_format = create_swapchain(vulkan_context, extent);
        create_framebuffers(vulkan_context, extent, surface_format);
        println!("[ResizeHandler::resize] New extent: {:?}", extent);

        // Update projection matrix in uniform buffer (Restore map_memory call)
        let ortho = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
        let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(uniform_allocation) }
            .unwrap()
            .cast::<f32>(); // Restore .cast() call
        unsafe { data_ptr.copy_from_nonoverlapping(ortho.to_cols_array().as_ptr(), 16) }
        unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(uniform_allocation) };
        println!("[ResizeHandler::resize] Projection matrix updated");

        warn!("[ResizeHandler::resize] Vertex resizing logic skipped (Needs ECS implementation)");

        // Re-record command buffers
        record_command_buffers(vulkan_context, renderables, pipeline_layout, descriptor_set, extent);
        println!("[ResizeHandler::resize] Command buffers re-recorded");
    }
}