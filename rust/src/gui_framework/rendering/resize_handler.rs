use ash::vk;
use crate::Vertex;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use bevy_math::Mat4;

pub struct ResizeHandler;

impl ResizeHandler {
    pub fn resize(
        vulkan_context: &mut VulkanContext,
        scene: &mut Scene,
        renderables: &mut Vec<Renderable>,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
        width: u32,
        height: u32,
        uniform_allocation: &mut vk_mem::Allocation,
    ) {
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

        let ortho = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
        let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(uniform_allocation) }
            .unwrap()
            .cast::<f32>();
        unsafe { data_ptr.copy_from_nonoverlapping(ortho.to_cols_array().as_ptr(), 16) }
        unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(uniform_allocation) };

        for (renderable, obj) in renderables.iter_mut().zip(scene.pool.iter_mut()) {
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
        record_command_buffers(vulkan_context, renderables, pipeline_layout, descriptor_set, extent);
    }
}