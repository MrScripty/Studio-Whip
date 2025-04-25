use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
// Removed direct import of cleanup_swapchain_resources, it's called by ResizeHandler
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use bevy_math::Mat4;
use bevy_log::{info, warn, error};
use crate::RenderCommandData; // from lib.rs

pub struct Renderer {
    buffer_manager: BufferManager,
    // Store pool and layout needed for cleanup/buffer manager
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, surface_format);
        info!("[Renderer::new] Framebuffers created");

        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);
        info!("[Renderer::new] PipelineManager created (temporarily)");

        // Store layout in VulkanContext for BufferManager access
        platform.pipeline_layout = Some(pipeline_mgr.pipeline_layout);
        info!("[Renderer::new] PipelineLayout stored in VulkanContext");

        let buffer_mgr = BufferManager::new(
            platform, // Pass &mut VulkanContext
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set_layout,
            pipeline_mgr.descriptor_pool,
        );
        info!("[Renderer::new] BufferManager created");

        // Store pool and set_layout in Renderer for cleanup
        let descriptor_pool = pipeline_mgr.descriptor_pool;
        let descriptor_set_layout = pipeline_mgr.descriptor_set_layout;
        // pipeline_mgr goes out of scope here, its layout is moved to platform

        // Update global projection UBO (BufferManager owns the buffer/allocation)
        let initial_logical_width = extent.width as f32; // Use the extent passed to Renderer::new
        let initial_logical_height = extent.height as f32;
        unsafe {
            let proj = Mat4::orthographic_rh(0.0, initial_logical_width, 0.0, initial_logical_height, -1.0, 1.0);
            // We are flipping Y beacuse Bevy coord space uses +Y and Vulkan uses -Y 
            let flip_y = Mat4::from_scale(bevy_math::Vec3::new(1.0, -1.0, 1.0));
            let proj_matrix = flip_y * proj;

            let allocator = platform.allocator.as_ref().unwrap();
            let info = allocator.get_allocation_info(&buffer_mgr.uniform_allocation);
            bevy_log::info!("Renderer::new: Writing initial projection for logical extent {}x{}, Matrix:\n{:?}", initial_logical_width, initial_logical_height, proj_matrix);
            // Use get_allocation_info for persistently mapped buffer
            if !info.mapped_data.is_null() {
                let data_ptr = info.mapped_data.cast::<f32>();
                data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
                // No need to unmap
            } else {
                error!("[Renderer::new] Failed to get mapped pointer for initial uniform buffer update.");
                // Attempt map/unmap as fallback? Or panic?
                // For now, log error.
            }
        }
        info!("[Renderer::new] Global projection UBO buffer updated (Descriptor set update deferred to BufferManager)");

        // --- Create Command Pool (Once) ---
        // Command buffers will be allocated later in record_command_buffers if needed
        platform.command_pool = Some(unsafe {
            let queue_family_index = platform.queue_family_index
                .expect("Queue family index not set in VulkanContext");
            platform.device.as_ref().unwrap().create_command_pool(
                &vk::CommandPoolCreateInfo {
                    s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                    // Allow resetting individual command buffers or the whole pool
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index,
                    ..Default::default()
                },
                None,
            )
        }.expect("Failed to create command pool"));
        info!("[Renderer::new] Command pool created");


        // Create sync objects
        platform.image_available_semaphore = Some(unsafe {
            platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).expect("Failed to create image available semaphore")
        });
        platform.render_finished_semaphore = Some(unsafe {
             platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).expect("Failed to create render finished semaphore")
        });
        platform.fence = Some(unsafe {
            platform.device.as_ref().unwrap().create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                    flags: vk::FenceCreateFlags::SIGNALED, // Start signaled
                    ..Default::default()
                }, None).expect("Failed to create fence")
        });
        info!("[Renderer::new] Sync objects created");
        info!("[Renderer::new] Finished");

        Self {
            buffer_manager: buffer_mgr,
            descriptor_pool, // Store for cleanup
            descriptor_set_layout, // Store for cleanup
        }
    }

    // Accept &mut VulkanContext
    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) {
        info!("[Renderer::resize_renderer] Called with width: {}, height: {}", width, height);
        // Prevent resizing to 0x0 which causes Vulkan errors
        if width == 0 || height == 0 {
            warn!("[Renderer::resize_renderer] Ignoring resize to zero dimensions.");
            return;
        }
        let logical_extent = vk::Extent2D { width, height };
        ResizeHandler::resize(
            vulkan_context,
            logical_extent,
            &mut self.buffer_manager.uniform_allocation, // Pass the allocation for the UBO update
        );
        // Note: Command buffers will be re-allocated inside record_command_buffers
        // if the framebuffer count changed, which it shouldn't during typical resize.
        // If swapchain image count changes, this needs more handling.
    }

    // --- Modified render signature ---
    // Accept &mut VulkanContext and render commands
    pub fn render(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) {
        // --- Clone handles needed *after* the mutable borrow ---
        // Clone the ash::Device handle (cheap)
        let device = platform.device.as_ref().unwrap().clone();
        // Clone other handles (cheap) - Add check for queue
        let Some(queue) = platform.queue else {
            warn!("[Renderer::render] Queue is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(queue) = platform.queue else {
            warn!("[Renderer::render] Queue is None, likely during cleanup. Skipping frame.");
            return;
        };
        // Check if swapchain resources are still valid, might be None during cleanup
        let Some(swapchain_loader) = platform.swapchain_loader.as_ref().cloned() else {
            warn!("[Renderer::render] Swapchain loader is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(swapchain) = platform.swapchain else {
            warn!("[Renderer::render] Swapchain is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(image_available_semaphore) = platform.image_available_semaphore else {
            warn!("[Renderer::render] Image available semaphore is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(render_finished_semaphore) = platform.render_finished_semaphore else {
            warn!("[Renderer::render] Render finished semaphore is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(fence) = platform.fence else {
            warn!("[Renderer::render] Fence is None, likely during cleanup. Skipping frame.");
            return;
        };
        // Prefix unused allocator
        let _allocator = platform.allocator.as_ref().unwrap(); // Needed for buffer updates

        // --- Wait for previous frame's fence ---
        // This ensures the GPU is finished with the command buffer and resources
        // from the *last* time this image index was used before we reset/reuse them.
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        // Reset the fence *before* submitting new work that will signal it
        unsafe { device.reset_fences(&[fence]) }.unwrap();


        // --- Prepare Buffers/Descriptors (Call BufferManager) ---
        // Now it's safe to update descriptor sets as the GPU is done with the previous frame
        // This call takes the mutable borrow of platform
        let prepared_draw_data = self.buffer_manager.prepare_frame_resources(
            platform, // Pass mutable platform here
            render_commands, // Pass the commands from rendering_system
        );
        // Mutable borrow of platform for buffer manager ends here


        // --- Acquire Swapchain Image ---
        let acquire_result = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        };

        let image_index = match acquire_result {
            Ok((index, suboptimal)) => {
                if suboptimal {
                    warn!("[Renderer::render] Swapchain suboptimal during acquire.");
                    // TODO: Trigger resize handling here? Or just continue?
                }
                index
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during acquire. Triggering resize.");
                // Trigger resize explicitly
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
                return; // Skip rest of the frame, resize will handle recreation
            }
            Err(e) => panic!("Failed to acquire swapchain image: {:?}", e),
        };
        // We still need mutable access to platform to update current_image
        // This is okay because the mutable borrow for buffer manager ended.
        platform.current_image = image_index as usize;


        // --- Re-Record Command Buffer for the acquired image index ---
        // This now happens *after* acquiring the image index and *after* waiting on the fence.
        // It also resets the command pool/buffer internally.
        record_command_buffers(
            platform, // Pass mutable platform here again
            &prepared_draw_data, // Pass the prepared data from buffer manager
            platform.pipeline_layout.expect("Pipeline layout missing for command recording"), // Get layout from platform
            platform.current_swap_extent,
        );
        // Mutable borrow for command buffer recording ends here.


        // Ensure command buffer exists for the acquired image index
        if platform.current_image >= platform.command_buffers.len() {
             error!(
                 "[Renderer::render] Image index {} out of bounds for command buffers (len {}). Skipping submit.",
                 platform.current_image,
                 platform.command_buffers.len()
             );
             // This might happen if resize occurred but command buffers weren't recreated yet.
             // The allocation logic in record_command_buffers should handle this now.
             return; // Avoid panic
        }

        // --- Submit Queue ---
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            // Use the command buffer for the current image index, which was just recorded
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            ..Default::default()
        };
        // Use cloned device handle
        if let Err(e) = unsafe { device.queue_submit(queue, &[submit_info], fence) } {
             error!("[Renderer::render] Failed to submit queue: {:?}", e);
             // Don't panic here, let present handle potential OOD
             // return; // Optionally return early
        }


        // --- Present Queue ---
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            wait_semaphore_count: 1,
            p_wait_semaphores: &render_finished_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &(platform.current_image as u32),
            ..Default::default()
        };
        // Use cloned swapchain_loader handle
        let present_result = unsafe { swapchain_loader.queue_present(queue, &present_info) };

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    warn!("[Renderer::render] Swapchain suboptimal during present.");
                    // Trigger resize explicitly
                    self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present. Triggering resize.");
                // Trigger resize explicitly
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
            }
            Err(e) => panic!("Failed to present swapchain image: {:?}", e),
        }
    }

    // Accept &mut self and &mut VulkanContext
    pub fn cleanup(&mut self, platform: &mut VulkanContext) { // Changed to &mut self
        info!("[Renderer::cleanup] Called (&mut self)");
        // Clone device handle early if needed, but cleanup methods might take platform directly
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone();

        // Ensure GPU is idle before destroying anything
        unsafe { device.device_wait_idle().unwrap(); }
        info!("[Renderer::cleanup] Device idle.");

        // Call cleanup on BufferManager first (destroys buffers, pipelines, shaders)
        self.buffer_manager.cleanup(
            platform, // Pass &mut VulkanContext
        );
        info!("[Renderer::cleanup] BufferManager cleanup finished.");

        // Cleanup layout and pool stored in Renderer/Platform
        unsafe {
            if let Some(layout) = platform.pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
                 info!("[Renderer::cleanup] Pipeline layout destroyed");
            }
            // Use pool/set_layout stored in self
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            info!("[Renderer::cleanup] Descriptor pool and set layout destroyed");
        }

        // Cleanup swapchain resources (Framebuffers, Views, Swapchain, RenderPass)
        // Use the dedicated cleanup function
        crate::gui_framework::rendering::swapchain::cleanup_swapchain_resources(platform);
        info!("[Renderer::cleanup] Swapchain resources cleanup finished.");


        // Cleanup remaining resources (Sync objects, Command Pool)
        unsafe {
            if let Some(sema) = platform.image_available_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(sema) = platform.render_finished_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(fen) = platform.fence.take() { device.destroy_fence(fen, None); }
            info!("[Renderer::cleanup] Sync objects destroyed.");

            // Cleanup command pool *after* waiting for idle and *before* device destroy
            if let Some(pool) = platform.command_pool.take() {
                // Command buffers should be implicitly freed by pool destruction,
                // but explicit free doesn't hurt if needed. They are empty now anyway.
                if !platform.command_buffers.is_empty() {
                    // device.free_command_buffers(pool, &platform.command_buffers); // Optional explicit free
                    platform.command_buffers.clear(); // Clear the vec
                }
                device.destroy_command_pool(pool, None); // Now destroy the pool
                info!("[Renderer::cleanup] Command pool destroyed.");
            }
        }

        // Note: VulkanContext itself (device, instance, allocator) is cleaned up
        // by the main cleanup_system calling vulkan_setup::cleanup_vulkan

        info!("[Renderer::cleanup] Finished");
    }
}