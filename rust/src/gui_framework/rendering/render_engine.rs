use ash::vk;
use std::marker::PhantomData;
use crate::Vertex;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use glam::Mat4;
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager; // Add this line

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

    pub fn update_offset(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator, index: usize, offset: [f32; 2]) {
        let renderable = &mut self.vulkan_renderables[index];
        unsafe {
            let data_ptr = allocator
                .map_memory(&mut renderable.offset_allocation)
                .unwrap()
                .cast::<f32>();
            data_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
            allocator.unmap_memory(&mut renderable.offset_allocation);

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: renderable.offset_uniform,
                offset: 0,
                range: std::mem::size_of::<[f32; 2]>() as u64,
            };
            device.update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: renderable.descriptor_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_info,
                p_texel_buffer_view: std::ptr::null(),
                _marker: PhantomData,
            }], &[]);
        }
    }

    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) {
        let device = vulkan_context.device.as_ref().unwrap();
        unsafe { device.device_wait_idle().unwrap() };

        for &framebuffer in &vulkan_context.framebuffers {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_render_pass(vulkan_context.render_pass.take().unwrap(), None) };
        for &view in &vulkan_context.image_views {
            unsafe { device.destroy_image_view(view, None) };
        }
        if let Some(swapchain) = vulkan_context.swapchain.take() {
            unsafe { vulkan_context.swapchain_loader.as_ref().unwrap().destroy_swapchain(swapchain, None) };
        }

        let extent = vk::Extent2D { width, height };
        let surface_format = create_swapchain(vulkan_context, extent);
        create_framebuffers(vulkan_context, extent, surface_format);
        println!("New extent: {:?}", extent);

        let ortho = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0).to_cols_array();
        let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(&mut self.uniform_allocation) }
            .unwrap()
            .cast::<f32>();
        unsafe { data_ptr.copy_from_nonoverlapping(ortho.as_ptr(), ortho.len()) };
        unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(&mut self.uniform_allocation) };

        for (renderable, obj) in self.vulkan_renderables.iter_mut().zip(scene.pool.iter_mut()) {
            let mut new_vertices = Vec::new();
            if renderable.on_window_resize_scale {
                new_vertices = vec![
                    Vertex { position: [0.0, 0.0] },
                    Vertex { position: [0.0, height as f32] },
                    Vertex { position: [width as f32, height as f32] },
                    Vertex { position: [width as f32, 0.0] },
                ];
            } else if renderable.on_window_resize_move {
                let new_center_x = renderable.center_ratio[0] * width as f32;
                let new_center_y = renderable.center_ratio[1] * height as f32;
                let half_width = renderable.fixed_size[0] / 2.0;
                let half_height = renderable.fixed_size[1] / 2.0;

                if renderable.vertex_count == 4 {
                    new_vertices = vec![
                        Vertex { position: [new_center_x - half_width, new_center_y - half_height] },
                        Vertex { position: [new_center_x - half_width, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y - half_height] },
                    ];
                } else if renderable.vertex_count == 3 {
                    new_vertices = vec![
                        Vertex { position: [new_center_x - half_width, new_center_y - half_height] },
                        Vertex { position: [new_center_x, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y - half_height] },
                    ];
                }
            }

            let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(&mut renderable.vertex_allocation) }
                .unwrap()
                .cast::<Vertex>();
            unsafe { data_ptr.copy_from_nonoverlapping(new_vertices.as_ptr(), new_vertices.len()) };
            unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(&mut renderable.vertex_allocation) };
            obj.vertices = new_vertices; // Sync Scene vertices
        }

        scene.update_dimensions(width, height);
        record_command_buffers(vulkan_context, &self.vulkan_renderables, self.pipeline_layout, self.descriptor_set, extent);
    }

    pub fn update_instance_offset(&mut self, _device: &ash::Device, allocator: &vk_mem::Allocator, object_index: usize, instance_id: usize, offset: [f32; 2]) {
        let renderable = &mut self.vulkan_renderables[object_index];
        if let Some(ref mut instance_allocation) = renderable.instance_allocation {
            unsafe {
                let data_ptr = allocator.map_memory(instance_allocation).unwrap().cast::<f32>();
                let offset_ptr = data_ptr.add(instance_id * 2); // Each instance offset is 2 f32s
                offset_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
                allocator.unmap_memory(instance_allocation);
            }
        }
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
            self.update_offset(device, allocator, i, obj.offset);
            for (j, instance) in obj.instances.iter().enumerate() {
                self.update_instance_offset(device, allocator, i, j, instance.offset);
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