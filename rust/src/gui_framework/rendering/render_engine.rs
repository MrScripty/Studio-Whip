use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
// Removed direct import of cleanup_swapchain_resources, it's called by ResizeHandler
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use bevy_log::{warn, error};
use crate::{RenderCommandData, VulkanContextResource};  // from lib.rs
use crate::{PreparedTextDrawData, GlobalProjectionUboResource}; // Added TextVertex and TextRenderCommandData
 // Added for text vertex buffer allocation
 // Added for render signature

pub struct Renderer {
    buffer_manager: BufferManager,
    // Store pool and layouts needed for cleanup
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout, // For shapes (Set 0)
    pub text_descriptor_set_layout: vk::DescriptorSetLayout, // For text atlas sampler (Set 1)
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, surface_format);

        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);

        // Store layouts in VulkanContext for access by other systems
        platform.shape_pipeline_layout = Some(pipeline_mgr.shape_pipeline_layout);
        platform.text_pipeline_layout = Some(pipeline_mgr.text_pipeline_layout);

        // Create BufferManager - Pass only needed layout/pool
        let buffer_mgr = BufferManager::new(
            platform, // Pass &mut VulkanContext
            pipeline_mgr.per_entity_layout,
            pipeline_mgr.descriptor_pool,
        );

        // Store pool and set_layout in Renderer for cleanup
        let descriptor_pool = pipeline_mgr.descriptor_pool;
        let per_entity_layout = pipeline_mgr.per_entity_layout;
        let atlas_layout = pipeline_mgr.atlas_layout;

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

        // Initialize Renderer struct
        Self {
            buffer_manager: buffer_mgr,
            descriptor_pool,
            descriptor_set_layout: per_entity_layout,
            text_descriptor_set_layout: atlas_layout,
        }
    }

    // Accept &mut VulkanContext and GlobalProjectionUboResource
    pub fn resize_renderer(
        &mut self,
        vk_context_res: &VulkanContextResource, // <-- Accept the resource
        width: u32,
        height: u32,
        global_ubo_res: &GlobalProjectionUboResource,
    ) {
        // Prevent resizing to 0x0 which causes Vulkan errors
        if width == 0 || height == 0 {
            warn!("[Renderer::resize_renderer] Ignoring resize to zero dimensions.");
            return;
        }
        let logical_extent = vk::Extent2D { width, height };

        // Lock VulkanContext *inside* resize_renderer
        if let Ok(mut vk_ctx_guard) = vk_context_res.0.lock() {
            ResizeHandler::resize(
                &mut vk_ctx_guard, // <-- Pass mutable context guard
                logical_extent,
            );
            // vk_ctx_guard lock released here
        } else {
            warn!("[Renderer::resize_renderer] Could not lock VulkanContext. Resize skipped.");
        }
    }

    // Accept prepared shape and text draw data, and the global UBO resource
    pub fn render(
        &mut self,
        vk_context_res: &VulkanContextResource, // <-- Accept resource
        shape_commands: &[RenderCommandData],
        prepared_text_draws: &[PreparedTextDrawData],
        global_ubo_res: &GlobalProjectionUboResource,
    ) {
        // --- Lock context once at the beginning to get handles and perform operations ---
        let Ok(platform_guard) = vk_context_res.0.lock() else {
            warn!("[Renderer::render] Could not lock VulkanContext. Skipping frame.");
            return;
        };

        // --- Get handles needed (cloning cheap handles like Device is fine) ---
        let device = match platform_guard.device.as_ref() {
            Some(d) => d.clone(),
            None => { warn!("[Renderer::render] Device is None. Skipping frame."); return; }
        };
        let Some(queue) = platform_guard.queue else { warn!("[Renderer::render] Queue is None. Skipping frame."); return; };
        let Some(swapchain_loader) = platform_guard.swapchain_loader.as_ref().cloned() else { warn!("[Renderer::render] Swapchain loader is None. Skipping frame."); return; };
        let Some(swapchain) = platform_guard.swapchain else { warn!("[Renderer::render] Swapchain is None. Skipping frame."); return; };
        let Some(image_available_semaphore) = platform_guard.image_available_semaphore else { warn!("[Renderer::render] Image available semaphore is None. Skipping frame."); return; };
        let Some(render_finished_semaphore) = platform_guard.render_finished_semaphore else { warn!("[Renderer::render] Render finished semaphore is None. Skipping frame."); return; };
        let Some(fence) = platform_guard.fence else { warn!("[Renderer::render] Fence is None. Skipping frame."); return; };
        // Allocator reference is obtained within prepare_frame_resources now
        // Get current extent *before* dropping the lock, needed for resize calls below
        let current_extent = platform_guard.current_swap_extent;

        // --- Drop the lock temporarily before potentially long waits ---
        // We'll re-acquire it when needed for mutable operations.
        // This is crucial to avoid holding the lock during vkWaitForFences.
        drop(platform_guard);

        // --- Wait for previous frame's fence ---
        // This ensures the GPU is finished with the command buffer and resources
        // from the *last* time this image index was used before we reset/reuse them.
        if let Err(e) = unsafe { device.wait_for_fences(&[fence], true, u64::MAX) } { // Use if let for error handling
            error!("[Renderer::render] Error waiting for fence: {:?}. Skipping frame.", e);
            // We might be stuck if we can't wait. Consider returning?
            return;
        }
        // Reset the fence *before* submitting new work that will signal it
        if let Err(e) = unsafe { device.reset_fences(&[fence]) } {
             error!("[Renderer::render] Error resetting fence: {:?}. Skipping frame.", e);
             return; // Avoid proceeding with a potentially broken fence state
        }

        // --- Prepare Shape Buffers/Descriptors (Call BufferManager) ---
        // Re-acquire lock for mutable access needed by BufferManager
        let Ok(mut platform_guard) = vk_context_res.0.lock() else {
             warn!("[Renderer::render] Could not re-lock VulkanContext for prepare_frame_resources. Skipping frame.");
             return;
        };
        let prepared_shape_draws = self.buffer_manager.prepare_frame_resources(
            &mut platform_guard,
            shape_commands,
            global_ubo_res,
        );
        // Drop lock again before acquiring swapchain image
        drop(platform_guard);

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
                // Trigger resize explicitly - pass the resource
                self.resize_renderer(vk_context_res, current_extent.width, current_extent.height, global_ubo_res);
                return; // Skip rest of the frame, resize will handle recreation
            }
            Err(e) => panic!("Failed to acquire swapchain image: {:?}", e),
        };

        // Re-acquire lock to update current_image and record command buffer
        let Ok(mut platform_guard) = vk_context_res.0.lock() else {
             warn!("[Renderer::render] Could not re-lock VulkanContext for command buffer recording. Skipping frame.");
             return;
        };

        // Update current_image
        let width = platform_guard.current_swap_extent.width; // Get width/height for resize call below
        let height = platform_guard.current_swap_extent.height;
        platform_guard.current_image = image_index as usize;

        // --- Re-Record Command Buffer for the acquired image index ---
        // --- Re-Record Command Buffer for the acquired image index ---
        // Get extent *before* the mutable borrow for record_command_buffers
        let extent_for_recording = platform_guard.current_swap_extent;
        record_command_buffers(
            &mut platform_guard, // Pass mutable guard
            &prepared_shape_draws,
            prepared_text_draws,
            extent_for_recording, // Pass the pre-fetched extent
        );

        // Get the command buffer handle *before* dropping the lock
        let current_command_buffer = platform_guard.command_buffers[platform_guard.current_image];
        // Get the image index *before* dropping the lock
        let current_image_index = platform_guard.current_image as u32;

        // Drop lock before submitting queue
        drop(platform_guard);
        // Mutable borrow for command buffer recording ends here.

        // --- Submit Queue ---
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            // Use the command buffer handle obtained before dropping the lock
            p_command_buffers: &current_command_buffer,
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
            p_image_indices: &current_image_index,
            ..Default::default()
        };
        // Use cloned swapchain_loader handle
        let present_result = unsafe { swapchain_loader.queue_present(queue, &present_info) };

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    warn!("[Renderer::render] Swapchain suboptimal during present.");
                    // Trigger resize explicitly - pass the resource
                    self.resize_renderer(vk_context_res, current_extent.width, current_extent.height, global_ubo_res);
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present. Triggering resize.");
                // Trigger resize explicitly - pass the resource
                self.resize_renderer(vk_context_res, current_extent.width, current_extent.height, global_ubo_res);
            }
            Err(e) => panic!("Failed to present swapchain image: {:?}", e),
        }
    }

    // Accept &mut self and &mut VulkanContext
    pub fn cleanup(&mut self, platform: &mut VulkanContext) { // Changed to &mut self
        // Clone device handle early if needed, but cleanup methods might take platform directly
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone();

        // Ensure GPU is idle before destroying anything
        unsafe { device.device_wait_idle().unwrap(); }

        // Call cleanup on BufferManager first (destroys buffers, pipelines, shaders)
        self.buffer_manager.cleanup(
            platform, // Pass &mut VulkanContext
        );

        // Ceanup of text vertex buffer handled by cleanup_trigger_system
        // Cleanup layouts stored in Renderer/Platform
        unsafe {
            // Destroy layouts stored in VulkanContext
            if let Some(layout) = platform.shape_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
            }
             if let Some(layout) = platform.text_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
            }
            // Use pool/set_layouts stored in self
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); // Per-entity layout
            device.destroy_descriptor_set_layout(self.text_descriptor_set_layout, None); // Atlas layout
        }

        // Cleanup of text pipeline handled by cleanup_trigger_system
        // Cleanup swapchain resources (Framebuffers, Views, Swapchain, RenderPass)
        // Use the dedicated cleanup function
        crate::gui_framework::rendering::swapchain::cleanup_swapchain_resources(platform);

        // Cleanup remaining resources (Sync objects, Command Pool)
        unsafe {
            if let Some(sema) = platform.image_available_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(sema) = platform.render_finished_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(fen) = platform.fence.take() { device.destroy_fence(fen, None); }

            // Cleanup command pool *after* waiting for idle and *before* device destroy
            if let Some(pool) = platform.command_pool.take() {
                // Command buffers should be implicitly freed by pool destruction,
                // but explicit free doesn't hurt if needed. They are empty now anyway.
                if !platform.command_buffers.is_empty() {
                    // device.free_command_buffers(pool, &platform.command_buffers); // Optional explicit free
                    platform.command_buffers.clear(); // Clear the vec
                }
                device.destroy_command_pool(pool, None); // Now destroy the pool
            }
        }
        // Note: VulkanContext itself (device, instance, allocator) is cleaned up
        // by the main cleanup_system calling vulkan_setup::cleanup_vulkan
    }
}