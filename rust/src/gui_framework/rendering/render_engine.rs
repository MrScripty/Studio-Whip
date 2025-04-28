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
use crate::{TextVertex, PreparedTextDrawData, GlobalProjectionUboResource}; // Added TextVertex and TextRenderCommandData
use vk_mem::{Allocation, Alloc}; // Added for text vertex buffer allocation
use crate::GlyphAtlasResource; // Added for render signature
use crate::gui_framework::rendering::shader_utils;

pub struct Renderer {
    buffer_manager: BufferManager,
    // Store pool and layouts needed for cleanup
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout, // For shapes
    pub text_descriptor_set_layout: vk::DescriptorSetLayout,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, surface_format);
        info!("[Renderer::new] Framebuffers created");

        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);
        info!("[Renderer::new] PipelineManager created (temporarily)");

        // Store layouts in VulkanContext for BufferManager access and text resource creation
        platform.shape_pipeline_layout = Some(pipeline_mgr.shape_pipeline_layout);
        platform.text_pipeline_layout = Some(pipeline_mgr.text_pipeline_layout);
        info!("[Renderer::new] Shape and Text PipelineLayouts stored in VulkanContext");

        // Create BufferManager - Pass only needed layout/pool
        let buffer_mgr = BufferManager::new(
            platform, // Pass &mut VulkanContext
            pipeline_mgr.shape_descriptor_set_layout, // Pass only shape layout
            pipeline_mgr.descriptor_pool,             // Pass pool
        );
        info!("[Renderer::new] BufferManager created");

        // Store pool and set_layout in Renderer for cleanup
        let descriptor_pool = pipeline_mgr.descriptor_pool;
        // Store the shape layout, as BufferManager uses it
        let descriptor_set_layout = pipeline_mgr.shape_descriptor_set_layout;
        // Also store the text layout for potential future cleanup needs? Or let VulkanContext own it?
        // Let's store both shape and text layouts in Renderer for cleanup for now.
        let text_descriptor_set_layout = pipeline_mgr.text_descriptor_set_layout;
        // pipeline_mgr goes out of scope here, its layouts are moved to platform/Renderer

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

        // Initialize Renderer struct
        Self {
            buffer_manager: buffer_mgr,
            descriptor_pool, // Store for cleanup
            descriptor_set_layout, // Store shape layout for cleanup
            text_descriptor_set_layout, // Store text layout for cleanup
            // Removed text-specific fields
        }
    }

    // Accept &mut VulkanContext and GlobalProjectionUboResource
    pub fn resize_renderer(
        &mut self,
        vulkan_context: &mut VulkanContext,
        width: u32,
        height: u32,
        global_ubo_res: &GlobalProjectionUboResource, // Pass global UBO resource
    ) {
        info!("[Renderer::resize_renderer] Called with width: {}, height: {}", width, height);
        // Prevent resizing to 0x0 which causes Vulkan errors
        if width == 0 || height == 0 {
            warn!("[Renderer::resize_renderer] Ignoring resize to zero dimensions.");
            return;
        }
        let logical_extent = vk::Extent2D { width, height };
        // ResizeHandler no longer needs the allocation directly, it's handled by handle_resize_system
        ResizeHandler::resize(
            vulkan_context,
            logical_extent,
            // Removed allocation parameter
        );
    }

    // Accept prepared shape and text draw data, and the global UBO resource
    pub fn render(
        &mut self,
        platform: &mut VulkanContext,
        shape_commands: &[RenderCommandData],
        prepared_text_draws: &[PreparedTextDrawData], // Pass prepared text data
        global_ubo_res: &GlobalProjectionUboResource, // Pass global UBO resource
    ) {
        // --- Clone handles needed *after* the mutable borrow ---
        // Clone the ash::Device handle (cheap)
        let device = platform.device.as_ref().unwrap().clone();
        // Clone other handles (cheap) - Add check for queue
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


        // --- Prepare Shape Buffers/Descriptors (Call BufferManager) ---
        // Pass the global UBO resource to BufferManager
        let prepared_shape_draws = self.buffer_manager.prepare_frame_resources(
            platform, // Pass mutable platform here
            shape_commands, // Pass the shape commands
            global_ubo_res, // Pass the global UBO resource
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
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height, global_ubo_res);
                return; // Skip rest of the frame, resize will handle recreation
            }
            Err(e) => panic!("Failed to acquire swapchain image: {:?}", e),
        };
        // We still need mutable access to platform to update current_image
        // This is okay because the mutable borrow for buffer manager ended.
        platform.current_image = image_index as usize;


        // --- Re-Record Command Buffer for the acquired image index ---
        record_command_buffers(
            platform, // Pass mutable platform here again
            &prepared_shape_draws,
            prepared_text_draws, // Pass the prepared text draw data slice
            platform.current_swap_extent,
            // Removed text_vertex_buffer, glyph_atlas_descriptor_set, text_pipeline
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
                    self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height, global_ubo_res);
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present. Triggering resize.");
                // Trigger resize explicitly
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height, global_ubo_res);
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

        // Ceanup of text vertex buffer handled by cleanup_trigger_system

        // Cleanup layouts stored in Renderer/Platform
        unsafe {
            // Destroy layouts stored in VulkanContext
            if let Some(layout) = platform.shape_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
                 info!("[Renderer::cleanup] Shape pipeline layout destroyed");
            }
             if let Some(layout) = platform.text_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
                 info!("[Renderer::cleanup] Text pipeline layout destroyed");
            }
            // Use pool/set_layouts stored in self
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); // Shape layout
            device.destroy_descriptor_set_layout(self.text_descriptor_set_layout, None); // Text layout
            info!("[Renderer::cleanup] Descriptor pool and set layouts destroyed");
        }

        // Cleanup of text pipeline handled by cleanup_trigger_system

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