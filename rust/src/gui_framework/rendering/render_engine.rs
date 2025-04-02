use ash::vk;
use std::marker::PhantomData;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use crate::gui_framework::rendering::renderable::Renderable; // Keep Renderable import if needed elsewhere
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use crate::gui_framework::event_bus::{EventHandler, BusEvent};
use std::sync::{Arc, Mutex};
use std::any::Any;
use glam::Mat4; // Import Mat4

pub struct Renderer {
    pipeline_manager: PipelineManager,
    buffer_manager: BufferManager,
    pending_instance_updates: Arc<Mutex<Vec<(usize, usize, [f32; 2])>>>,
}

impl EventHandler for Renderer {
     fn handle(&mut self, event: &BusEvent) {
        match event {
            BusEvent::InstanceAdded(object_id, instance_id, offset) => {
                let mut queue = self.pending_instance_updates.lock().unwrap();
                queue.push((*object_id, *instance_id, *offset));
            }
            _ => {}
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}


impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self {
        println!("[Renderer::new] Start");
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);
        println!("[Renderer::new] Framebuffers created");
        // 1. Create PipelineManager
        let pipeline_mgr = PipelineManager::new(platform, scene);
        println!("[Renderer::new] PipelineManager created");
        // 2. Create BufferManager, passing resources from PipelineManager
        let mut buffer_mgr = BufferManager::new( // Make mutable to update uniform buffer allocation
            platform,
            scene,
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set_layout,
            pipeline_mgr.descriptor_pool,
        );
        println!("[Renderer::new] BufferManager created");
        // Update the *global* projection descriptor set (binding 0) held by PipelineManager
        unsafe {
            let proj_matrix = Mat4::orthographic_rh(0.0, extent.width as f32, extent.height as f32, 0.0, -1.0, 1.0).to_cols_array();
             let allocator = platform.allocator.as_ref().unwrap();
             // Map the uniform buffer allocation held by BufferManager
             let data_ptr = allocator.map_memory(&mut buffer_mgr.uniform_allocation)
                 .expect("Failed to map uniform buffer for projection update")
                 .cast::<f32>();
             data_ptr.copy_from_nonoverlapping(proj_matrix.as_ptr(), proj_matrix.len());
             allocator.unmap_memory(&mut buffer_mgr.uniform_allocation); // Unmap original mut ref

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: buffer_mgr.uniform_buffer,
                offset: 0,
                range: std::mem::size_of_val(&proj_matrix) as u64,
            };
            platform.device.as_ref().unwrap().update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: pipeline_mgr.descriptor_set, // Update PipelineManager's set
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &buffer_info,
                ..Default::default()
            }], &[]);
        }
        println!("[Renderer::new] Global projection UBO updated");

        // Record command buffers using renderables from BufferManager
        record_command_buffers(
            platform,
            &buffer_mgr.renderables,
            pipeline_mgr.pipeline_layout,
            pipeline_mgr.descriptor_set, // Pass PipelineManager's set
            extent
        );
        println!("[Renderer::new] Initial command buffers recorded");
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
        println!("[Renderer::new] Sync objects created");
        println!("[Renderer::new] Finished"); // Log end
        Self {
            pipeline_manager: pipeline_mgr,
            buffer_manager: buffer_mgr, // Store the created BufferManager
            pending_instance_updates: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) {
        println!("[Renderer::render] Frame start");
        // Delegate resize, passing necessary components
        ResizeHandler::resize(
            vulkan_context,
            scene,
            &mut self.buffer_manager.renderables, // Pass renderables from BufferManager
            self.pipeline_manager.pipeline_layout,
            self.pipeline_manager.descriptor_set, // Pass global descriptor set
            width,
            height,
            &mut self.buffer_manager.uniform_allocation, // Pass uniform allocation from BufferManager
        );
    }

    pub fn render(&mut self, platform: &mut VulkanContext, scene: &Scene) {
        let device = platform.device.as_ref().unwrap();
        let queue = platform.queue.unwrap();
        let swapchain_loader = platform.swapchain_loader.as_ref().unwrap();
        let swapchain = platform.swapchain.unwrap();
        let image_available_semaphore = platform.image_available_semaphore.unwrap();
        let render_finished_semaphore = platform.render_finished_semaphore.unwrap();
        let fence = platform.fence.unwrap();
        let allocator = platform.allocator.as_ref().unwrap();
        println!("[Renderer::render] Frame start");

        // Process pending instance updates
        let updates_to_process = {
             let mut queue_guard = self.pending_instance_updates.lock().unwrap();
             queue_guard.drain(..).collect::<Vec<_>>()
        };
        for (object_id, instance_id, offset) in updates_to_process {
            BufferManager::update_instance_buffer(
                &mut self.buffer_manager.renderables, // Use BufferManager's renderables
                device,
                allocator,
                object_id,
                instance_id,
                offset,
            );
        }

        // Update existing object/instance offsets
        for (i, obj) in scene.pool.iter().enumerate() {
             if i < self.buffer_manager.renderables.len() {
                BufferManager::update_offset(&mut self.buffer_manager.renderables, device, allocator, i, obj.offset);
                for (j, instance) in obj.instances.iter().enumerate() {
                     if let Some(_) = self.buffer_manager.renderables[i].instance_buffer {
                         if j < self.buffer_manager.renderables[i].instance_count as usize {
                            BufferManager::update_instance_offset(&mut self.buffer_manager.renderables, device, allocator, i, j, instance.offset);
                         }
                     }
                }
            }
        }

        println!("[Renderer::render] Waiting for fence...");
        // Render sequence
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        println!("[Renderer::render] Resetting fence...");
        unsafe { device.reset_fences(&[fence]) }.unwrap();
        println!("[Renderer::render] Acquiring next image...");

        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        }.unwrap();
        platform.current_image = image_index as usize;
        println!("[Renderer::render] Acquired image index: {}", image_index);

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
        println!("[Renderer::render] Command buffer submitted");

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

    // Cleanup resources in correct order
    pub fn cleanup(self, platform: &mut VulkanContext) {
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone(); // Clone ash::Device
        let allocator = platform.allocator.as_ref().expect("Allocator not available for cleanup").clone(); // Clone Arc<Allocator>

        // Ensure device is idle before destroying anything
        unsafe { device.device_wait_idle().unwrap(); }

        // Cleanup BufferManager first, passing refs and the pool
        self.buffer_manager.cleanup(
            &device,
            &allocator, // Pass allocator ref
            self.pipeline_manager.descriptor_pool
        );

        // Then cleanup PipelineManager, passing device ref
        self.pipeline_manager.cleanup(&device);

        // Cleanup swapchain resources managed by Renderer/VulkanContext
        // Use the local 'device' reference obtained earlier
        if let Some(swapchain_loader) = platform.swapchain_loader.take() {
            unsafe {
                // Destroy sync objects
                if let Some(sema) = platform.image_available_semaphore.take() { device.destroy_semaphore(sema, None); }
                if let Some(sema) = platform.render_finished_semaphore.take() { device.destroy_semaphore(sema, None); }
                if let Some(fen) = platform.fence.take() { device.destroy_fence(fen, None); }

                // Destroy command pool and buffers
                if let Some(pool) = platform.command_pool.take() {
                    if !platform.command_buffers.is_empty() {
                        device.free_command_buffers(pool, &platform.command_buffers);
                        platform.command_buffers.clear();
                    }
                    device.destroy_command_pool(pool, None);
                }

                // Destroy framebuffers and render pass
                for fb in platform.framebuffers.drain(..) { device.destroy_framebuffer(fb, None); }
                if let Some(rp) = platform.render_pass.take() { device.destroy_render_pass(rp, None); }

                // Destroy swapchain and image views
                if let Some(sc) = platform.swapchain.take() { swapchain_loader.destroy_swapchain(sc, None); }
                for view in platform.image_views.drain(..) { device.destroy_image_view(view, None); }
            }
        // Note: We don't destroy the device or allocator here, that happens in vulkan_setup::cleanup_vulkan
        }
    }
}