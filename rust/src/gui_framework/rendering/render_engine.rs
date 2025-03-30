use ash::vk;
use std::marker::PhantomData;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use glam::Mat4;
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;

pub struct Renderer {
    vulkan_renderables: Vec<Renderable>,
    pipeline_layout: vk::PipelineLayout,
    uniform_buffer: vk::Buffer,
    uniform_allocation: vk_mem::Allocation,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);
    
        let pipeline_mgr = PipelineManager::new(platform, scene);
        let buffer_mgr = BufferManager::new(platform, scene, pipeline_mgr.pipeline_layout);

        unsafe {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: buffer_mgr.uniform_buffer,
                offset: 0,
                range: std::mem::size_of::<Mat4>() as u64,
            };
            platform.device.as_ref().unwrap().update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: pipeline_mgr.descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_info,
                p_texel_buffer_view: std::ptr::null(),
                _marker: PhantomData,
            }], &[]);
        }

        record_command_buffers(platform, &buffer_mgr.renderables, pipeline_mgr.pipeline_layout, pipeline_mgr.descriptor_set, extent);

        platform.image_available_semaphore = Some(unsafe {
            match platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None) {
                Ok(semaphore) => semaphore,
                Err(e) => {
                    println!("Failed to create image available semaphore: {:?}", e);
                    panic!("Semaphore creation failed");
                }
            }
        });
        platform.render_finished_semaphore = Some(unsafe {
            match platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None) {
                Ok(semaphore) => semaphore,
                Err(e) => {
                    println!("Failed to create render finished semaphore: {:?}", e);
                    panic!("Semaphore creation failed");
                }
            }
        });
        platform.fence = Some(unsafe {
            match platform.device.as_ref().unwrap().create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::FenceCreateFlags::SIGNALED,
                    _marker: PhantomData,
                },
                None,
            ) {
                Ok(fence) => fence,
                Err(e) => {
                    println!("Failed to create fence: {:?}", e);
                    panic!("Fence creation failed");
                }
            }
        });

        Self {
            vulkan_renderables: buffer_mgr.renderables,
            pipeline_layout: pipeline_mgr.pipeline_layout,
            uniform_buffer: buffer_mgr.uniform_buffer,
            uniform_allocation: buffer_mgr.uniform_allocation,
            descriptor_set_layout: buffer_mgr.descriptor_set_layout,
            descriptor_pool: buffer_mgr.descriptor_pool,
            descriptor_set: pipeline_mgr.descriptor_set,
        }
    }

    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) {
        ResizeHandler::resize(
            vulkan_context,
            scene,
            &mut self.vulkan_renderables,
            self.pipeline_layout,
            self.descriptor_set,
            width,
            height,
            &mut self.uniform_allocation,
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

        for (i, obj) in scene.pool.iter().enumerate() {
            BufferManager::update_offset(&mut self.vulkan_renderables, device, allocator, i, obj.offset);
            for (j, instance) in obj.instances.iter().enumerate() {
                BufferManager::update_instance_offset(&mut self.vulkan_renderables, device, allocator, i, j, instance.offset);
            }
        }

        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        unsafe { device.reset_fences(&[fence]) }.unwrap();

        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        }.unwrap();
        platform.current_image = image_index as usize;

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT as *const _,
            command_buffer_count: 1,
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            _marker: PhantomData,
        };
        unsafe { device.queue_submit(queue, &[submit_info], fence) }.unwrap();

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &render_finished_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &(platform.current_image as u32),
            p_results: std::ptr::null_mut(),
            _marker: PhantomData,
        };
        unsafe { swapchain_loader.queue_present(queue, &present_info) }.unwrap();
    }

    pub fn cleanup(self, platform: &mut VulkanContext) {
        PipelineManager::cleanup(
            PipelineManager {
                pipeline_layout: self.pipeline_layout,
                descriptor_set_layout: self.descriptor_set_layout,
                descriptor_pool: self.descriptor_pool,
                descriptor_set: self.descriptor_set,
            },
            platform,
        );
        
        BufferManager::cleanup(
            BufferManager {
                uniform_buffer: self.uniform_buffer,
                uniform_allocation: self.uniform_allocation,
                renderables: self.vulkan_renderables,
                descriptor_set_layout: self.descriptor_set_layout,
                descriptor_pool: self.descriptor_pool,
            },
            platform,
        );

        let device = platform.device.as_ref().unwrap();
        let swapchain_loader = platform.swapchain_loader.take().unwrap();
    
        unsafe {
            device.device_wait_idle().unwrap();
    
            device.destroy_semaphore(platform.image_available_semaphore.take().unwrap(), None);
            device.destroy_semaphore(platform.render_finished_semaphore.take().unwrap(), None);
            device.destroy_fence(platform.fence.take().unwrap(), None);
    
            let command_pool = platform.command_pool.take().unwrap();
            device.free_command_buffers(command_pool, &platform.command_buffers);
            device.destroy_command_pool(command_pool, None);
    
            for &framebuffer in &platform.framebuffers {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_render_pass(platform.render_pass.take().unwrap(), None);
    
            swapchain_loader.destroy_swapchain(platform.swapchain.take().unwrap(), None);
            for &view in &platform.image_views {
                device.destroy_image_view(view, None);
            }
        }
    }
}