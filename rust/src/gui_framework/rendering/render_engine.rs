use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
// Removed direct import of cleanup_swapchain_resources, it's called by ResizeHandler
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::text_renderer::TextRenderer;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use bevy_log::{warn, error, info};
use crate::{RenderCommandData, VulkanContextResource, TextRenderingResources}; 
use crate::gui_framework::plugins::core::TextLayoutInfo;
use crate::GlobalProjectionUboResource;


pub struct Renderer {
    buffer_manager: BufferManager,
    // Store pool and layouts needed for cleanup
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout, // For shapes (Set 0)
    pub text_descriptor_set_layout: vk::DescriptorSetLayout, // For text atlas sampler (Set 1)
    text_renderer: TextRenderer,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        // --- Create Command Pool (Once) ---
        // Moved EARLIER: Command pool must exist before create_framebuffers if it allocates command buffers.
        platform.command_pool = Some(unsafe {
            let queue_family_index = platform.queue_family_index
                .expect("Queue family index not set in VulkanContext for command pool creation");
            let device = platform.device.as_ref()
                .expect("Device not available for command pool creation");
            device.create_command_pool(
                &vk::CommandPoolCreateInfo {
                    s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index,
                    p_next: std::ptr::null(),
                    _marker: std::marker::PhantomData,
                },
                None,
            )
        }.expect("Failed to create command pool"));
        info!("[Renderer::new] Command pool created."); // Log after creation
    
        // Now create swapchain and framebuffers (which uses the command pool)
        let surface_format = create_swapchain(platform, extent);
        info!("[Renderer::new] Swapchain created. platform.command_buffers len before create_framebuffers: {}", platform.command_buffers.len()); // Existing log
        create_framebuffers(platform, surface_format);
        info!("[Renderer::new] Framebuffers (and command buffers) created. platform.command_buffers len: {}", platform.command_buffers.len());
    
    
        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);
    
        // Store layouts in VulkanContext for access by other systems
        platform.shape_pipeline_layout = Some(pipeline_mgr.shape_pipeline_layout);
        platform.text_pipeline_layout = Some(pipeline_mgr.text_pipeline_layout);
    
        // Create BufferManager - Pass only needed layout/pool
        let buffer_mgr = BufferManager::new(
            platform, 
            pipeline_mgr.per_entity_layout,
            pipeline_mgr.descriptor_pool,
        );
    
        // Create TextRenderer
        let text_renderer_instance = TextRenderer::new(
            pipeline_mgr.descriptor_pool, // Use the same pool
            pipeline_mgr.per_entity_layout, // Pass the layout for Set 0
        );
    
        // Store pool and set_layout in Renderer for cleanup
        let descriptor_pool = pipeline_mgr.descriptor_pool;
        let per_entity_layout = pipeline_mgr.per_entity_layout;
        let atlas_layout = pipeline_mgr.atlas_layout;
    
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
                    p_next: std::ptr::null(),
                    _marker: std::marker::PhantomData,
                }, None).expect("Failed to create fence")
        });
    
        // Initialize Renderer struct
        Self {
            buffer_manager: buffer_mgr,
            text_renderer: text_renderer_instance,
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
        vk_context_res: &VulkanContextResource,
        shape_commands: &[RenderCommandData],
        text_layout_infos: &[TextLayoutInfo],
        global_ubo_res: &GlobalProjectionUboResource,
        text_global_res: &TextRenderingResources,
    ) {
        // --- DIAGNOSTIC: Lock context ONCE at the beginning for preparations ---
        let mut platform_guard = match vk_context_res.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("[Renderer::render] DIAGNOSTIC: Initial lock failed (poisoned): {:?}. Skipping frame.", poisoned);
                return;
            }
        };
        info!("[Renderer::render] DIAGNOSTIC: Initial lock for preparations acquired.");

        // --- Get handles needed (cloning cheap handles like Device is fine) ---
        let device = match platform_guard.device.as_ref() {
            Some(d) => d.clone(),
            None => { warn!("[Renderer::render] DIAGNOSTIC: Device is None. Skipping frame."); drop(platform_guard); return; }
        };
        let queue = match platform_guard.queue {
            Some(q) => q,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Queue is None. Skipping frame."); drop(platform_guard); return; }
        };
        let swapchain_loader = match platform_guard.swapchain_loader.as_ref().cloned() {
            Some(sl) => sl,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Swapchain loader is None. Skipping frame."); drop(platform_guard); return; }
        };
        let swapchain = match platform_guard.swapchain {
            Some(s) => s,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Swapchain is None. Skipping frame."); drop(platform_guard); return; }
        };
        let image_available_semaphore = match platform_guard.image_available_semaphore {
            Some(s) => s,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Image available semaphore is None. Skipping frame."); drop(platform_guard); return; }
        };
        let render_finished_semaphore = match platform_guard.render_finished_semaphore {
            Some(s) => s,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Render finished semaphore is None. Skipping frame."); drop(platform_guard); return; }
        };
        let fence = match platform_guard.fence {
            Some(f) => f,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Fence is None. Skipping frame."); drop(platform_guard); return; }
        };
        let current_extent = platform_guard.current_swap_extent; // Keep for resize
        let allocator_arc = match platform_guard.allocator.clone() {
            Some(alloc) => alloc,
            None => { warn!("[Renderer::render] DIAGNOSTIC: Allocator is None. Skipping frame."); drop(platform_guard); return; }
        };

        // --- Prepare Shape Buffers/Descriptors (Call BufferManager) ---
        info!("[Renderer::render] DIAGNOSTIC: Calling BufferManager::prepare_frame_resources...");
        let prepared_shape_draws = self.buffer_manager.prepare_frame_resources(
            &mut platform_guard,
            shape_commands,
            global_ubo_res,
        );
        info!("[Renderer::render] DIAGNOSTIC: BufferManager::prepare_frame_resources returned {} draws.", prepared_shape_draws.len());

        // --- Prepare Text Draws (Call TextRenderer) ---
        info!("[Renderer::render] DIAGNOSTIC: Calling TextRenderer::prepare_text_draws...");
        let prepared_text_draws = self.text_renderer.prepare_text_draws(
            &device,
            &allocator_arc,
            text_layout_infos,
            global_ubo_res,
            text_global_res,
        );
        info!("[Renderer::render] DIAGNOSTIC: TextRenderer::prepare_text_draws returned {} draws.", prepared_text_draws.len());

        // --- DIAGNOSTIC: Drop the preparation lock BEFORE fence wait ---
        info!("[Renderer::render] DIAGNOSTIC: Dropping preparation lock.");
        drop(platform_guard); // Explicitly drop the guard

        // --- Wait for previous frame's fence ---
        info!("[Renderer::render] DIAGNOSTIC: About to wait_for_fences.");
        if let Err(e) = unsafe { device.wait_for_fences(&[fence], true, u64::MAX) } {
            error!("[Renderer::render] DIAGNOSTIC: Error waiting for fence: {:?}. Skipping frame.", e);
            return;
        }
        info!("[Renderer::render] DIAGNOSTIC: wait_for_fences successful.");
        if let Err(e) = unsafe { device.reset_fences(&[fence]) } {
             error!("[Renderer::render] DIAGNOSTIC: Error resetting fence: {:?}. Skipping frame.", e);
             return;
        }
        info!("[Renderer::render] DIAGNOSTIC: reset_fences successful.");

        // --- Acquire Swapchain Image ---
        info!("[Renderer::render] DIAGNOSTIC: About to acquire_next_image.");
        let acquire_result = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        };
        let image_index = match acquire_result {
            Ok((index, suboptimal)) => {
                if suboptimal { warn!("[Renderer::render] DIAGNOSTIC: Swapchain suboptimal during acquire."); }
                index
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] DIAGNOSTIC: Swapchain out of date during acquire. Triggering resize.");
                match vk_context_res.0.lock() { // Attempt to lock for resize
                    Ok(mut temp_guard) => ResizeHandler::resize(&mut temp_guard, current_extent),
                    Err(_) => error!("[Renderer::render] DIAGNOSTIC: Failed to lock context for OOD resize during acquire!"),
                }
                return;
            }
            Err(e) => { error!("[Renderer::render] DIAGNOSTIC: Failed to acquire swapchain image: {:?}", e); return; }
        };
        info!("[Renderer::render] DIAGNOSTIC: Acquired image_index: {}", image_index);

        // --- DIAGNOSTIC: Re-acquire lock for command recording and presentation ---
        info!("[Renderer::render] DIAGNOSTIC: Attempting FINAL lock of VulkanContext before command recording...");
        let mut platform_guard = match vk_context_res.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("[Renderer::render] DIAGNOSTIC: FINAL lock failed (poisoned): {:?}. Skipping frame.", poisoned);
                return;
            }
        };
        info!("[Renderer::render] DIAGNOSTIC: FINAL lock of VulkanContext successful.");

        platform_guard.current_image = image_index as usize;
        info!("[Renderer::render] DIAGNOSTIC: platform_guard.current_image set to: {}", platform_guard.current_image);

        let extent_for_recording = platform_guard.current_swap_extent;
        info!("[Renderer::render] DIAGNOSTIC: extent_for_recording fetched: {}x{}", extent_for_recording.width, extent_for_recording.height);
        
        let command_buffer_to_submit = platform_guard.command_buffers[platform_guard.current_image]; // Get before potential drop

        info!("[Renderer::render] DIAGNOSTIC: About to call record_command_buffers for image_index: {}", platform_guard.current_image);
        record_command_buffers(
            &mut platform_guard,
            &prepared_shape_draws,
            &prepared_text_draws,
            extent_for_recording,
        );
        info!("[Renderer::render] DIAGNOSTIC: record_command_buffers finished for image_index: {}", image_index);

        // --- Submit Queue (Keep platform_guard locked) ---
        info!("[Renderer::render] DIAGNOSTIC: About to queue_submit for image_index: {}", image_index);
        let wait_semaphores = [image_available_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [render_finished_semaphore];
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stages.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &command_buffer_to_submit,
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            _marker: std::marker::PhantomData, // Added marker
        };
        if let Err(e) = unsafe { device.queue_submit(queue, &[submit_info], fence) } {
             error!("[Renderer::render] DIAGNOSTIC: Failed to submit queue: {:?}", e);
             drop(platform_guard); // Drop guard before returning on error
             return;
        }
        info!("[Renderer::render] DIAGNOSTIC: queue_submit finished for image_index: {}", image_index);

        // --- Present Queue (Keep platform_guard locked) ---
        info!("[Renderer::render] DIAGNOSTIC: About to queue_present for image_index: {}", image_index);
        let swapchains = [swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: signal_semaphores.len() as u32, // Wait on render_finished_semaphore
            p_wait_semaphores: signal_semaphores.as_ptr(),
            swapchain_count: swapchains.len() as u32,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: image_indices.as_ptr(),
            p_results: std::ptr::null_mut(), // Not checking individual swapchain results
            _marker: std::marker::PhantomData, // Added marker
        };
        let present_result = unsafe { swapchain_loader.queue_present(queue, &present_info) };
        
        // --- DIAGNOSTIC: Drop the lock AFTER present attempt ---
        info!("[Renderer::render] DIAGNOSTIC: Dropping FINAL lock post-present attempt.");
        drop(platform_guard);

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    warn!("[Renderer::render] DIAGNOSTIC: Swapchain suboptimal during present.");
                    match vk_context_res.0.lock() { // Attempt to lock for resize
                        Ok(mut temp_guard) => ResizeHandler::resize(&mut temp_guard, current_extent),
                        Err(_) => error!("[Renderer::render] DIAGNOSTIC: Failed to lock context for OOD resize during present! (suboptimal)"),
                    }
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] DIAGNOSTIC: Swapchain out of date during present. Triggering resize.");
                 match vk_context_res.0.lock() { // Attempt to lock for resize
                    Ok(mut temp_guard) => ResizeHandler::resize(&mut temp_guard, current_extent),
                    Err(_) => error!("[Renderer::render] DIAGNOSTIC: Failed to lock context for OOD resize during present! (OOD_KHR)"),
                }
            }
            Err(e) => { error!("[Renderer::render] DIAGNOSTIC: Failed to present swapchain image: {:?}", e); /* Consider panic or return */ }
        }
        info!("[Renderer::render] DIAGNOSTIC: queue_present finished for image_index: {}", image_index);
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

        // --- Cleanup TextRenderer (which cleans its cached resources) ---
        let allocator_arc_for_text_cleanup = platform.allocator.clone().expect("Allocator missing for text renderer cleanup");
        self.text_renderer.cleanup(&device, &allocator_arc_for_text_cleanup);
        info!("[Renderer::cleanup] TextRenderer cleanup called.");

        // --- Cleanup Layouts and Pool ---
        unsafe {
            // Destroy layouts stored in VulkanContext
            if let Some(layout) = platform.shape_pipeline_layout.take() { device.destroy_pipeline_layout(layout, None); }
            if let Some(layout) = platform.text_pipeline_layout.take() { device.destroy_pipeline_layout(layout, None); }
            // Destroy layouts stored in self
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); // Per-entity layout
            device.destroy_descriptor_set_layout(self.text_descriptor_set_layout, None); // Atlas layout
            // Destroy pool *after* freeing sets and cleaning BufferManager
            device.destroy_descriptor_pool(self.descriptor_pool, None);
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