use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
// Removed EventBus/EventHandler imports
// Removed Arc, Mutex, Any imports (no longer used here)
use bevy_math::Mat4;
use bevy_log::warn; // Added warn import

pub struct Renderer {
    pipeline_manager: PipelineManager,
    buffer_manager: BufferManager,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        println!("[Renderer::new] Start (ECS Migration)");
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);
        println!("[Renderer::new] Framebuffers created");

        let pipeline_mgr = PipelineManager::new(platform);
        println!("[Renderer::new] PipelineManager created");

        let mut buffer_mgr = BufferManager::new(
            platform,
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set_layout,
            pipeline_mgr.descriptor_pool,
        );
        println!("[Renderer::new] BufferManager created (Needs ECS rework)");

        // Update global projection UBO
        unsafe {
            let proj_matrix = Mat4::orthographic_rh(0.0, extent.width as f32, extent.height as f32, 0.0, -1.0, 1.0);
            let allocator = platform.allocator.as_ref().unwrap();
            let data_ptr = allocator.map_memory(&mut buffer_mgr.uniform_allocation)
                .expect("Failed to map uniform buffer for projection update")
                .cast::<f32>();
            data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
            allocator.unmap_memory(&mut buffer_mgr.uniform_allocation);

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: buffer_mgr.uniform_buffer,
                offset: 0,
                range: std::mem::size_of::<Mat4>() as u64,
            };
            platform.device.as_ref().unwrap().update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: pipeline_mgr.descriptor_set,
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &buffer_info,
                ..Default::default()
            }], &[]);
        }
        println!("[Renderer::new] Global projection UBO updated");

        // Record initial command buffers (using empty renderables list for now)
        record_command_buffers(
            platform,
            &buffer_mgr.renderables, // Empty Vec
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set,
            extent
        );
        println!("[Renderer::new] Initial command buffers recorded (Using empty list)");

        // Create sync objects (Restore actual calls)
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
        println!("[Renderer::new] Sync objects created");
        println!("[Renderer::new] Finished");

        Self {
            pipeline_manager: pipeline_mgr,
            buffer_manager: buffer_mgr,
        }
    }

    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) {
        println!("[Renderer::resize_renderer] Called (ECS Migration)");
        ResizeHandler::resize(
            vulkan_context,
            &mut self.buffer_manager.renderables, // Still uses old renderables
            self.pipeline_manager.pipeline_layout,
            self.pipeline_manager.descriptor_set,
            width,
            height,
            &mut self.buffer_manager.uniform_allocation,
        );
    }

    pub fn render(&mut self, platform: &mut VulkanContext) {
        // NOTE: Needs complete rework for Step 7.
        let device = platform.device.as_ref().unwrap();
        let queue = platform.queue.unwrap();
        let swapchain_loader = platform.swapchain_loader.as_ref().unwrap();
        let swapchain = platform.swapchain.unwrap();
        let image_available_semaphore = platform.image_available_semaphore.unwrap();
        let render_finished_semaphore = platform.render_finished_semaphore.unwrap();
        let fence = platform.fence.unwrap();
        let _allocator = platform.allocator.as_ref().unwrap(); // Mark unused for now

        warn!("[Renderer::render] Skipping object offset updates (Needs ECS implementation)");

        // Render sequence
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        unsafe { device.reset_fences(&[fence]) }.unwrap();

        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        }.unwrap();
        platform.current_image = image_index as usize;

        // Restore SubmitInfo initializer
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            ..Default::default()
        };
        unsafe { device.queue_submit(queue, &[submit_info], fence) }.unwrap();

        // Restore PresentInfoKHR initializer
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            wait_semaphore_count: 1,
            p_wait_semaphores: &render_finished_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &(platform.current_image as u32),
            ..Default::default()
        };
        unsafe { swapchain_loader.queue_present(queue, &present_info) }.unwrap();
    }

    pub fn cleanup(self, platform: &mut VulkanContext) {
        println!("[Renderer::cleanup] Called");
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone();
        let allocator = platform.allocator.as_ref().expect("Allocator not available for cleanup").clone();

        unsafe { device.device_wait_idle().unwrap(); }

        self.buffer_manager.cleanup(
            &device,
            &allocator,
            self.pipeline_manager.descriptor_pool
        );
        self.pipeline_manager.cleanup(&device);

        // Cleanup swapchain resources
        if let Some(_swapchain_loader) = platform.swapchain_loader.take() { // Mark unused
            unsafe { // Keep unsafe block as it contains destroy calls
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
                if let Some(sc) = platform.swapchain.take() { _swapchain_loader.destroy_swapchain(sc, None); } // Use _swapchain_loader
                for view in platform.image_views.drain(..) { device.destroy_image_view(view, None); }
            }
        }
        println!("[Renderer::cleanup] Finished");
    }
}