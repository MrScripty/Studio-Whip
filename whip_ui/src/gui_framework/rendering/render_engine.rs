use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::create_swapchain;
use crate::gui_framework::rendering::swapchain::create_framebuffers;
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
use crate::BufferManagerResource;
use bevy_ecs::prelude::Commands;
use std::sync::Mutex;
use std::sync::Arc;


pub struct Renderer {
    // Store pool and layouts needed for cleanup
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set_layout: vk::DescriptorSetLayout, // For shapes (Set 0)
    pub text_descriptor_set_layout: vk::DescriptorSetLayout, // For text atlas sampler (Set 1)
    text_renderer: TextRenderer,
}

impl Renderer {
    pub fn new(
        commands: &mut Commands,
        platform: &mut VulkanContext,
        extent: vk::Extent2D,
    ) -> Self {
        // --- Create Command Pool (Once) and store in VulkanContext ---
        // This must happen before create_framebuffers if it allocates command buffers from this pool.
        if platform.command_pool.is_none() {
            let queue_family_index = platform.queue_family_index
                .expect("Queue family index not set in VulkanContext for command pool creation");
            let device = platform.device.as_ref()
                .expect("Device not available for command pool creation");
            platform.command_pool = Some(unsafe {
                device.create_command_pool(
                    &vk::CommandPoolCreateInfo {
                        s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER, // Allows individual buffer reset
                        queue_family_index,
                        p_next: std::ptr::null(),
                        _marker: std::marker::PhantomData,
                    },
                    None,
                )
            }.expect("Failed to create command pool"));
            info!("[Renderer::new] Command pool created and stored in VulkanContext.");
        } else {
            info!("[Renderer::new] Command pool already exists in VulkanContext.");
        }

        // Create swapchain (populates VulkanContext swapchain fields)
        let surface_format = create_swapchain(platform, extent); // We need surface_format now
        info!("[Renderer::new] Swapchain created.");

        // Explicitly call create_framebuffers AFTER swapchain and its dependencies are set up
        create_framebuffers(platform, surface_format);
        info!("[Renderer::new] Framebuffers and command buffers created via explicit call.");
    
        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);
    
        // Store layouts in VulkanContext for access by other systems
        platform.shape_pipeline_layout = Some(pipeline_mgr.shape_pipeline_layout);
        platform.text_pipeline_layout = Some(pipeline_mgr.text_pipeline_layout);
    
        // Create BufferManager instance
        let buffer_manager_instance = BufferManager::new(
            platform,
            pipeline_mgr.per_entity_layout, // This is the layout for Set 0 (Global UBO, Transform UBO)
            pipeline_mgr.descriptor_pool,   // This is the shared pool
        );
        // Insert BufferManager as a resource using the passed-in commands
        commands.insert_resource(BufferManagerResource(Arc::new(Mutex::new(buffer_manager_instance))));
        info!("[Renderer::new] BufferManagerResource inserted.");
    
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

    pub fn render(
        &mut self,
        vk_context_res: &VulkanContextResource,
        buffer_manager_res: &BufferManagerResource,
        shape_commands: &[RenderCommandData],
        text_layout_infos: &[TextLayoutInfo],
        global_ubo_res: &GlobalProjectionUboResource,
        text_global_res: Option<&TextRenderingResources>, // TEMP: Accept Option
        mut debug_buffer: Option<&mut crate::gui_framework::debug::DebugRingBuffer>,
    ) {
        // --- Get essential handles that are relatively stable or cloneable ---
        // These are fetched once to avoid repeated locking if possible.
        // Device, Queue, Semaphores, Fence, Allocator can be cloned/copied.
        // Swapchain KHR and SwapchainLoader need careful handling due to resize.

        let (device, queue, image_available_semaphore, render_finished_semaphore, fence, allocator_arc, initial_swapchain_loader, initial_swapchain_khr, initial_current_extent) = {
            let temp_platform_guard = match vk_context_res.0.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("[Renderer::render] Initial lock to get handles failed (poisoned): {:?}. Skipping frame.", poisoned);
                    return;
                }
            };

            (
                temp_platform_guard.device.as_ref().expect("Device missing").clone(),
                temp_platform_guard.queue.expect("Queue missing"),
                temp_platform_guard.image_available_semaphore.expect("Image available semaphore missing"),
                temp_platform_guard.render_finished_semaphore.expect("Render finished semaphore missing"),
                temp_platform_guard.fence.expect("Fence missing"),
                temp_platform_guard.allocator.as_ref().expect("Allocator missing").clone(),
                temp_platform_guard.swapchain_loader.as_ref().expect("Swapchain loader missing").clone(),
                temp_platform_guard.swapchain.expect("Swapchain KHR missing"),
                temp_platform_guard.current_swap_extent,
            )
            // temp_platform_guard is dropped here
        };

        // --- 1. Wait for previous frame's fence ---
        if let Err(e) = unsafe { device.wait_for_fences(&[fence], true, u64::MAX) } {
            error!("[Renderer::render] Error waiting for fence: {:?}. Skipping frame.", e);
            return;
        }
        if let Err(e) = unsafe { device.reset_fences(&[fence]) } {
            error!("[Renderer::render] Error resetting fence: {:?}. Skipping frame.", e);
            return;
        }

        // --- 2. Acquire Swapchain Image ---
        // We use the initially fetched swapchain_loader and swapchain_khr.
        // If resize happens, these might become stale, but acquire_next_image handles ERROR_OUT_OF_DATE_KHR.
        let image_index = match unsafe {
            initial_swapchain_loader.acquire_next_image(initial_swapchain_khr, u64::MAX, image_available_semaphore, vk::Fence::null())
        } {
            Ok((index, suboptimal)) => {
                if suboptimal { warn!("[Renderer::render] Swapchain suboptimal during acquire."); }
                index
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during acquire. Triggering resize.");
                // Lock VulkanContext to perform resize
                match vk_context_res.0.lock() {
                    Ok(mut platform_guard) => ResizeHandler::resize(&mut platform_guard, initial_current_extent),
                    Err(_) => error!("[Renderer::render] Failed to lock context for OOD resize during acquire!"),
                }
                return; // Skip rest of the frame
            }
            Err(e) => {
                error!("[Renderer::render] Failed to acquire swapchain image: {:?}", e);
                return;
            }
        };

        // --- 3. Lock VulkanContext for the rest of the operations ---
        let mut platform_guard = match vk_context_res.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("[Renderer::render] Main lock failed (poisoned): {:?}. Skipping frame.", poisoned);
                return;
            }
        };

        platform_guard.current_image = image_index as usize;

        // --- 4. Prepare Frame Resources (Buffers, Descriptors) ---
        // Lock BufferManagerResource to call prepare_frame_resources
        let prepared_shape_draws = {
            // buffer_manager_res is passed as a parameter to Renderer::render
            let mut bm_guard = buffer_manager_res.0.lock().expect("Failed to lock BufferManagerResource in render");
            bm_guard.prepare_frame_resources(
                &mut platform_guard, 
                shape_commands,
                global_ubo_res,
            )
        }; // bm_guard dropped here

        let prepared_text_draws = match text_global_res { // Match the Option<&...> directly
            Some(text_res) => { // text_res is &TextRenderingResources here
                // Get debug device extension struct reference from locked context guard
                let debug_device_ext = platform_guard.debug_utils_device.as_ref(); // Get Option<&Device>
                self.text_renderer.prepare_text_draws(
                    &device, // Pass base device
                    &allocator_arc,
                    debug_device_ext, // Pass the Option<&Device>
                    text_layout_infos,
                    global_ubo_res,
                    text_res, // Pass the unwrapped &TextRenderingResources
                    debug_buffer.as_deref_mut(),
                )
            }
            None => Vec::new(), // Return empty if no text resources
        };

        // --- 5. Reset and Record Command Buffer ---
        let current_command_buffer = platform_guard.command_buffers[platform_guard.current_image];
        unsafe {
            device.reset_command_buffer(current_command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset command buffer");
        }

        record_command_buffers(
            &*platform_guard, // Pass &VulkanContext
            &prepared_shape_draws,
            &prepared_text_draws,
            platform_guard.current_swap_extent, // Get current extent from context
            debug_buffer,
        );

        // --- 6. Submit Queue ---
        let wait_semaphores = [image_available_semaphore]; // Semaphore to wait on
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT]; // Stage to wait at
        let signal_semaphores = [render_finished_semaphore]; // Semaphore to signal when done
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: wait_semaphores.len() as u32, // Set wait semaphore count
            p_wait_semaphores: wait_semaphores.as_ptr(),        // Point to wait semaphore
            p_wait_dst_stage_mask: wait_stages.as_ptr(),        // Point to wait stages
            command_buffer_count: 1,
            p_command_buffers: &current_command_buffer,
            signal_semaphore_count: signal_semaphores.len() as u32,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };
        if let Err(e) = unsafe { device.queue_submit(queue, &[submit_info], fence) } {
            error!("[Renderer::render] Failed to submit queue: {:?}", e);
            // platform_guard is dropped automatically when returning
            return;
        }

        // --- 7. Present Queue ---
        // Use the swapchain KHR from the locked context, as it might have been updated by a resize
        let current_swapchain_khr_for_present = platform_guard.swapchain.expect("Swapchain KHR missing for present");
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_wait_semaphores: signal_semaphores.as_ptr(),
            wait_semaphore_count: signal_semaphores.len() as u32,
            swapchain_count: 1,
            p_swapchains: &current_swapchain_khr_for_present, // Use current from context
            p_image_indices: &image_index,
            ..Default::default()
        };

        // Drop the main lock *before* calling queue_present, as queue_present can block
        // and we want to minimize lock duration. The swapchain_loader is cloned.
        // The current_swap_extent for resize handling is `initial_current_extent`.
        let swapchain_loader_for_present = platform_guard.swapchain_loader.as_ref().unwrap().clone();
        drop(platform_guard);

        let present_result = unsafe { swapchain_loader_for_present.queue_present(queue, &present_info) };

        match present_result {
            Ok(suboptimal) if suboptimal => {
                warn!("[Renderer::render] Swapchain suboptimal during present. Triggering resize.");
                match vk_context_res.0.lock() {
                    Ok(mut guard) => ResizeHandler::resize(&mut guard, initial_current_extent),
                    Err(_) => error!("[Renderer::render] Failed to lock context for OOD resize (suboptimal)!"),
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present. Triggering resize.");
                match vk_context_res.0.lock() {
                    Ok(mut guard) => ResizeHandler::resize(&mut guard, initial_current_extent),
                    Err(_) => error!("[Renderer::render] Failed to lock context for OOD resize (OOD_KHR)!"),
                }
            }
            Err(e) => error!("[Renderer::render] Failed to present swapchain image: {:?}", e),
            Ok(_) => {} // Success
        }
    }

    pub fn cleanup(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<vk_mem::Allocator>,
    ) {
        // The device_wait_idle is now handled by the caller (cleanup_trigger_system).

        // --- Cleanup TextRenderer ---
        self.text_renderer.cleanup(device, allocator);
        info!("[Renderer::cleanup] TextRenderer cleanup called.");

        // --- Cleanup Layouts and Pool ---
        // This function is now only responsible for resources owned by the Renderer struct.
        // The pipeline layouts stored in VulkanContext are cleaned by the main cleanup system.
        unsafe {
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            info!("[Renderer::cleanup] Destroyed shape descriptor set layout.");
            device.destroy_descriptor_set_layout(self.text_descriptor_set_layout, None);
            info!("[Renderer::cleanup] Destroyed text descriptor set layout.");
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            info!("[Renderer::cleanup] Destroyed descriptor pool.");
        }

        // NOTE: Cleanup of swapchain, sync objects, and command pool is now handled
        // by the main cleanup_trigger_system and cleanup_vulkan.
    }
}