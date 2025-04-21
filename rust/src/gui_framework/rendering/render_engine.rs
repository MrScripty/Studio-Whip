use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use bevy_math::Mat4;
use bevy_log::{info, warn, error}; // Use info, warn, and error
use crate::RenderCommandData; // Import RenderCommandData from lib.rs

pub struct Renderer {
    pipeline_manager: PipelineManager,
    buffer_manager: BufferManager,
    // Add extent field to store current swapchain dimensions
    current_extent: vk::Extent2D,
}

impl Renderer {
    // Accept &mut VulkanContext
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        info!("[Renderer::new] Start (ECS Migration)");
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);
        info!("[Renderer::new] Framebuffers created");

        let pipeline_mgr = PipelineManager::new(platform);
        info!("[Renderer::new] PipelineManager created");

        // BufferManager::new now takes &mut VulkanContext
        let mut buffer_mgr = BufferManager::new(
            platform, // Pass &mut VulkanContext
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set_layout,
            pipeline_mgr.descriptor_pool,
        );
        info!("[Renderer::new] BufferManager created (Needs ECS rework)");

        // Update global projection UBO (BufferManager owns the buffer/allocation)
        unsafe {
            let proj_matrix = Mat4::orthographic_rh(0.0, extent.width as f32, extent.height as f32, 0.0, -1.0, 1.0);
            let allocator = platform.allocator.as_ref().unwrap();
            let data_ptr = allocator.map_memory(&mut buffer_mgr.uniform_allocation) // Use buffer_mgr's allocation
                .expect("Failed to map uniform buffer for projection update")
                .cast::<f32>();
            data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
            allocator.unmap_memory(&mut buffer_mgr.uniform_allocation); // Use buffer_mgr's allocation
        }
        info!("[Renderer::new] Global projection UBO buffer updated (Descriptor set update deferred to BufferManager)");

        // --- Remove initial command buffer recording ---
        warn!("[Renderer::new] Skipping initial command buffer recording (will happen in render)");

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
            pipeline_manager: pipeline_mgr,
            buffer_manager: buffer_mgr,
            current_extent: extent, // Store initial extent
        }
    }

    // Accept &mut VulkanContext
    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) {
        info!("[Renderer::resize_renderer] Called (ECS Migration)");
        let new_extent = vk::Extent2D { width, height };
        ResizeHandler::resize(
            vulkan_context,
            new_extent,
            &mut self.buffer_manager.uniform_allocation,
        );
        // Update stored extent
        self.current_extent = new_extent;
        info!("[Renderer::resize_renderer] Stored extent updated to {:?}", self.current_extent);
    }

    // --- Modified render signature ---
    // Accept &mut VulkanContext and render commands
    pub fn render(&mut self, platform: &mut VulkanContext, _render_commands: &[RenderCommandData]) { // Prefix unused render_commands
        // --- Clone handles needed *after* the mutable borrow ---
        // Clone the ash::Device handle (cheap)
        let device = platform.device.as_ref().unwrap().clone();
        // Clone other handles (cheap)
        let queue = platform.queue.unwrap(); // vk::Queue is Copy
        let swapchain_loader = platform.swapchain_loader.as_ref().unwrap().clone(); // swapchain::Device is Clone
        let swapchain = platform.swapchain.unwrap(); // vk::SwapchainKHR is Copy
        let image_available_semaphore = platform.image_available_semaphore.unwrap(); // vk::Semaphore is Copy
        let render_finished_semaphore = platform.render_finished_semaphore.unwrap(); // vk::Semaphore is Copy
        let fence = platform.fence.unwrap(); // vk::Fence is Copy
        // Prefix unused allocator
        let _allocator = platform.allocator.as_ref().unwrap(); // Needed for buffer updates

        // --- Prepare Buffers/Descriptors (Call BufferManager) ---
        warn!("[Renderer::render] Skipping BufferManager resource preparation/update (Needs BufferManager rework)");
        // let prepared_draw_data = self.buffer_manager.prepare_frame_resources(
        //     platform, // Pass mutable platform here
        //     _render_commands,
        //     self.pipeline_manager.descriptor_set, // Pass global set for binding 0
        // );

        // --- Record Command Buffers Dynamically ---
        warn!("[Renderer::render] Recording command buffers with placeholder data (Needs BufferManager rework)");
        // The specific draw data (including per-entity descriptor sets)
        // needs to be generated by BufferManager first.
        let prepared_draw_data: Vec<crate::PreparedDrawData> = Vec::new(); // Placeholder
        // TODO: Call self.buffer_manager.prepare_frame_resources(...) here later
        record_command_buffers(
            platform, // mutable
            &prepared_draw_data,
            self.pipeline_manager.pipeline_layout, // Used for binding sets
            self.current_extent, // Use stored extent
        );
        // Mutable borrow of platform ends here

        // --- Render sequence (Uses cloned handles now) ---
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        unsafe { device.reset_fences(&[fence]) }.unwrap();

        let acquire_result = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        };

        let image_index = match acquire_result {
            Ok((index, _suboptimal)) => index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during acquire. Skipping frame.");
                return;
            }
            Err(e) => panic!("Failed to acquire swapchain image: {:?}", e),
        };
        // We still need mutable access to platform to update current_image
        // This is okay because the mutable borrow for record_command_buffers ended.
        platform.current_image = image_index as usize;

        // Ensure command buffer exists for the acquired image index
        if platform.current_image >= platform.command_buffers.len() {
             error!(
                 "[Renderer::render] Image index {} out of bounds for command buffers (len {}). Skipping submit.",
                 platform.current_image,
                 platform.command_buffers.len()
             );
             return; // Avoid panic
        }

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            // Still need immutable access to platform.command_buffers here
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            ..Default::default()
        };
        // Use cloned device handle
        if let Err(e) = unsafe { device.queue_submit(queue, &[submit_info], fence) } {
             error!("[Renderer::render] Failed to submit queue: {:?}", e);
             return;
        }


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
            Ok(_suboptimal) => { /* Success or suboptimal */ }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present.");
            }
            Err(e) => panic!("Failed to present swapchain image: {:?}", e),
        }
    }

    // Accept &mut self and &mut VulkanContext
    pub fn cleanup(&mut self, platform: &mut VulkanContext) { // Changed to &mut self
        info!("[Renderer::cleanup] Called (&mut self)");
        // Clone device handle early if needed, but cleanup methods might take platform directly
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone();

        unsafe { device.device_wait_idle().unwrap(); }

        // Call cleanup on managers using &mut self
        self.buffer_manager.cleanup(
            platform, // Pass &mut VulkanContext
            self.pipeline_manager.descriptor_pool // Pass pool needed by buffer_manager cleanup
        );
        self.pipeline_manager.cleanup(&device); // Pass cloned device handle

        // Cleanup swapchain resources (Remains the same, uses platform)
        if let Some(swapchain_loader) = platform.swapchain_loader.take() {
            unsafe {
                if let Some(sema) = platform.image_available_semaphore.take() { device.destroy_semaphore(sema, None); }
                if let Some(sema) = platform.render_finished_semaphore.take() { device.destroy_semaphore(sema, None); }
                if let Some(fen) = platform.fence.take() { device.destroy_fence(fen, None); }
                if let Some(pool) = platform.command_pool.take() {
                    if !platform.command_buffers.is_empty() {
                        device.free_command_buffers(pool, &platform.command_buffers);
                        platform.command_buffers.clear();
                    }
                    device.destroy_command_pool(pool, None);
                }
                for fb in platform.framebuffers.drain(..) { device.destroy_framebuffer(fb, None); }
                if let Some(rp) = platform.render_pass.take() { device.destroy_render_pass(rp, None); }
                if let Some(sc) = platform.swapchain.take() { swapchain_loader.destroy_swapchain(sc, None); }
                for view in platform.image_views.drain(..) { device.destroy_image_view(view, None); }
            }
        }
        info!("[Renderer::cleanup] Finished");
    }
}