use ash::vk;
use bevy_ecs::entity::Entity;
use bevy_log::{error, info, warn};
use bevy_math::Mat4;
use std::{collections::HashMap, sync::Arc}; // Added Arc here
use vk_mem::Alloc; // Corrected Alloc import
use crate::gui_framework::context::vulkan_setup::set_debug_object_name;
use ash::ext::debug_utils;

use crate::{
    gui_framework::{
        components::TextRenderData,
        plugins::core::TextLayoutInfo,
    },
    GlobalProjectionUboResource, PreparedTextDrawData, TextRenderingResources, TextVertex,
    // VulkanContextResource, // Not needed directly here, device/allocator passed in
};

pub struct TextRenderer {
    text_render_resources: HashMap<Entity, TextRenderData>,
    descriptor_pool: vk::DescriptorPool,
    per_entity_layout_set0: vk::DescriptorSetLayout,
}

impl TextRenderer {
    pub fn new(
        descriptor_pool: vk::DescriptorPool,
        per_entity_layout_set0: vk::DescriptorSetLayout,
    ) -> Self {
        Self {
            text_render_resources: HashMap::new(),
            descriptor_pool,
            per_entity_layout_set0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare_text_draws(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<vk_mem::Allocator>,
        debug_device_ext: Option<&debug_utils::Device>,
        text_layout_infos: &[TextLayoutInfo],
        global_ubo_res: &GlobalProjectionUboResource,
        text_global_res: &TextRenderingResources,
    ) -> Vec<PreparedTextDrawData> {
        info!("[TextRenderer::prepare_text_draws] Entered. text_layout_infos count: {}", text_layout_infos.len());
        if !text_layout_infos.is_empty() {
            let first_info = &text_layout_infos[0];
            info!("[TextRenderer::prepare_text_draws] First entity: {:?}, glyph count: {}", first_info.entity, first_info.layout.glyphs.len());
        }
        let mut prepared_text_draws: Vec<PreparedTextDrawData> = Vec::new();

        for layout_info in text_layout_infos {
            if !layout_info.visibility.is_visible() {
                continue;
            }

            let entity = layout_info.entity;
            info!("[TextRenderer::prepare_text_draws] Processing entity: {:?}, visibility: {}", entity, layout_info.visibility.0);
            let global_transform = layout_info.transform;
            let text_layout = &layout_info.layout;

            let mut relative_vertices: Vec<TextVertex> =
                Vec::with_capacity(text_layout.glyphs.len() * 6);
            for positioned_glyph in &text_layout.glyphs {
                let tl_rel = positioned_glyph.vertices[0];
                let tr_rel = positioned_glyph.vertices[1];
                let br_rel = positioned_glyph.vertices[2];
                let bl_rel = positioned_glyph.vertices[3];
                let uv_min = positioned_glyph.glyph_info.uv_min;
                let uv_max = positioned_glyph.glyph_info.uv_max;
                relative_vertices.push(TextVertex {
                    position: tl_rel.into(),
                    uv: [uv_min[0], uv_min[1]],
                });
                relative_vertices.push(TextVertex {
                    position: bl_rel.into(),
                    uv: [uv_min[0], uv_max[1]],
                });
                relative_vertices.push(TextVertex {
                    position: br_rel.into(),
                    uv: [uv_max[0], uv_max[1]],
                });
                relative_vertices.push(TextVertex {
                    position: tl_rel.into(),
                    uv: [uv_min[0], uv_min[1]],
                });
                relative_vertices.push(TextVertex {
                    position: br_rel.into(),
                    uv: [uv_max[0], uv_max[1]],
                });
                relative_vertices.push(TextVertex {
                    position: tr_rel.into(),
                    uv: [uv_max[0], uv_min[1]],
                });
            }
            let vertex_count = relative_vertices.len() as u32;
            info!("[TextRenderer::prepare_text_draws] Entity: {:?}, Calculated vertex_count: {}", entity, vertex_count);

            if vertex_count == 0 {
                if let Some(mut removed_data) = self.text_render_resources.remove(&entity) {
                    warn!("[TextRenderer] Cleaning up TextRenderData for entity {:?} with 0 vertices.", entity);
                    unsafe {
                        allocator.destroy_buffer(removed_data.transform_ubo, &mut removed_data.transform_alloc);
                        allocator.destroy_buffer(removed_data.vertex_buffer, &mut removed_data.vertex_alloc);
                        if removed_data.descriptor_set_0 != vk::DescriptorSet::null() {
                            if let Err(e) = device.free_descriptor_sets(self.descriptor_pool, &[removed_data.descriptor_set_0]) {
                                error!("[TextRenderer] Failed to free text descriptor set for {:?}: {:?}", entity, e);
                            }
                        }
                    }
                }
                continue;
            }

            let transform_matrix = global_transform.compute_matrix();

            if let Some(render_data) = self.text_render_resources.get_mut(&entity) {
                // Update Existing Entity
                unsafe {
                    let info = allocator.get_allocation_info(&render_data.transform_alloc);
                    if !info.mapped_data.is_null() {
                        info.mapped_data.cast::<f32>().copy_from_nonoverlapping(transform_matrix.to_cols_array().as_ptr(), 16);
                    } else {
                        error!("[TextRenderer] Transform UBO not mapped for update {:?}!", entity);
                    }
                }

                let current_capacity_bytes = allocator.get_allocation_info(&render_data.vertex_alloc).size;
                let current_capacity_vertices = (current_capacity_bytes / std::mem::size_of::<TextVertex>() as u64) as u32;

                if vertex_count > current_capacity_vertices {
                    warn!("[TextRenderer] Vertex count ({}) exceeds capacity ({}) for {:?}. Recreating vertex buffer.", vertex_count, current_capacity_vertices, entity);
                    unsafe {
                        allocator.destroy_buffer(render_data.vertex_buffer, &mut render_data.vertex_alloc);
                    }
                    let new_capacity = (vertex_count as f32 * 1.2).ceil() as u32; // Ensure capacity is at least vertex_count
                    let new_size_bytes = (std::mem::size_of::<TextVertex>() * new_capacity as usize) as u64;
                    let (new_buffer, new_alloc) = unsafe {
                        let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: new_size_bytes, usage: vk::BufferUsageFlags::VERTEX_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                        let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                        allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to recreate text vertex buffer")
                    };
                    render_data.vertex_buffer = new_buffer;
                    render_data.vertex_alloc = new_alloc;
                }
                render_data.vertex_count = vertex_count;
                unsafe {
                    let info = allocator.get_allocation_info(&render_data.vertex_alloc);
                    if !info.mapped_data.is_null() {
                        info.mapped_data.cast::<TextVertex>().copy_from_nonoverlapping(relative_vertices.as_ptr(), vertex_count as usize);
                    } else {
                        error!("[TextRenderer] Vertex buffer not mapped for update {:?}!", entity);
                    }
                }

                let transform_buffer_info = vk::DescriptorBufferInfo { buffer: render_data.transform_ubo, offset: 0, range: std::mem::size_of::<Mat4>() as u64 };
                let global_buffer_info = vk::DescriptorBufferInfo { buffer: global_ubo_res.buffer, offset: 0, range: std::mem::size_of::<Mat4>() as u64 };
                let writes = [
                    vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: render_data.descriptor_set_0, dst_binding: 0, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &global_buffer_info, ..Default::default() },
                    vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: render_data.descriptor_set_0, dst_binding: 1, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &transform_buffer_info, ..Default::default() },
                ];
                unsafe { device.update_descriptor_sets(&writes, &[]); }

                prepared_text_draws.push(PreparedTextDrawData {
                    pipeline: text_global_res.pipeline,
                    vertex_buffer: render_data.vertex_buffer,
                    vertex_count: render_data.vertex_count,
                    projection_descriptor_set: render_data.descriptor_set_0,
                    atlas_descriptor_set: text_global_res.atlas_descriptor_set,
                });
            } else {
                // Create New Entity Resources
                let initial_capacity = (vertex_count as f32 * 1.2).ceil() as u32; // Ensure capacity is at least vertex_count
                let vertex_buffer_size = (std::mem::size_of::<TextVertex>() * initial_capacity as usize) as u64;
                let (vertex_buffer, vertex_alloc) = unsafe {
                    let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: vertex_buffer_size, usage: vk::BufferUsageFlags::VERTEX_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                    let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                    allocator.create_buffer(&buffer_info, &allocation_info).expect("Failed to create text vertex buffer")
                };
                #[cfg(debug_assertions)]
                if let Some(debug_device_ext) = debug_device_ext { // Rename parameter for clarity
                    let mem_handle = allocator.get_allocation_info(&vertex_alloc).device_memory;
                    set_debug_object_name(debug_device_ext, vertex_buffer, vk::ObjectType::BUFFER, &format!("TextVertexBuffer_Entity{:?}", entity));
                    set_debug_object_name(debug_device_ext, mem_handle, vk::ObjectType::DEVICE_MEMORY, &format!("TextVertexBuffer_Entity{:?}_Mem", entity));
                }
                unsafe {
                    let info = allocator.get_allocation_info(&vertex_alloc);
                    if info.mapped_data.is_null() { // Check if mapped_data is null
                        error!("[TextRenderer] Newly created vertex buffer for {:?} is not mapped!", entity);
                    } else {
                        info.mapped_data.cast::<TextVertex>().copy_from_nonoverlapping(relative_vertices.as_ptr(), vertex_count as usize);
                    }
                }

                let buffer_info = vk::BufferCreateInfo { s_type: vk::StructureType::BUFFER_CREATE_INFO, size: std::mem::size_of::<Mat4>() as u64, usage: vk::BufferUsageFlags::UNIFORM_BUFFER, sharing_mode: vk::SharingMode::EXCLUSIVE, ..Default::default() };
                let allocation_info = vk_mem::AllocationCreateInfo { flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, usage: vk_mem::MemoryUsage::AutoPreferDevice, ..Default::default() };
                let (transform_ubo, transform_alloc) = unsafe {
                     allocator.create_buffer(&buffer_info, &allocation_info)
                }.expect("Failed to create text transform UBO");
                #[cfg(debug_assertions)]
                if let Some(debug_device_ext) = debug_device_ext { // Rename parameter for clarity
                    let mem_handle = allocator.get_allocation_info(&transform_alloc).device_memory;
                    set_debug_object_name(debug_device_ext, transform_ubo, vk::ObjectType::BUFFER, &format!("TextTransformUBO_Entity{:?}", entity));
                    set_debug_object_name(debug_device_ext, mem_handle, vk::ObjectType::DEVICE_MEMORY, &format!("TextTransformUBO_Entity{:?}_Mem", entity));
                }
                unsafe {
                    let info = allocator.get_allocation_info(&transform_alloc);
                    if info.mapped_data.is_null() { // Check if mapped_data is null
                         error!("[TextRenderer] Newly created transform UBO for {:?} is not mapped!", entity);
                    } else {
                        info.mapped_data.cast::<f32>().copy_from_nonoverlapping(transform_matrix.to_cols_array().as_ptr(), 16);
                    }
                }

                let set_layouts = [self.per_entity_layout_set0];
                let alloc_info = vk::DescriptorSetAllocateInfo { s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO, descriptor_pool: self.descriptor_pool, descriptor_set_count: 1, p_set_layouts: set_layouts.as_ptr(), ..Default::default() };
                let descriptor_set_0 = unsafe { device.allocate_descriptor_sets(&alloc_info).expect("Failed to allocate text descriptor set 0").remove(0) };

                let transform_buffer_info = vk::DescriptorBufferInfo { buffer: transform_ubo, offset: 0, range: std::mem::size_of::<Mat4>() as u64 };
                let global_buffer_info = vk::DescriptorBufferInfo { buffer: global_ubo_res.buffer, offset: 0, range: std::mem::size_of::<Mat4>() as u64 };
                let writes = [
                    vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: descriptor_set_0, dst_binding: 0, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &global_buffer_info, ..Default::default() },
                    vk::WriteDescriptorSet { s_type: vk::StructureType::WRITE_DESCRIPTOR_SET, dst_set: descriptor_set_0, dst_binding: 1, descriptor_count: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, p_buffer_info: &transform_buffer_info, ..Default::default() },
                ];
                unsafe { device.update_descriptor_sets(&writes, &[]); }

                let new_render_data = TextRenderData {
                    vertex_count,
                    vertex_buffer,
                    vertex_alloc,
                    transform_ubo,
                    transform_alloc,
                    descriptor_set_0,
                };

                prepared_text_draws.push(PreparedTextDrawData {
                    pipeline: text_global_res.pipeline,
                    vertex_buffer: new_render_data.vertex_buffer,
                    vertex_count: new_render_data.vertex_count,
                    projection_descriptor_set: new_render_data.descriptor_set_0,
                    atlas_descriptor_set: text_global_res.atlas_descriptor_set,
                });

                self.text_render_resources.insert(entity, new_render_data);
            }
        }
        prepared_text_draws
    }

    pub fn cleanup(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<vk_mem::Allocator>, // Use Arc here
    ) {
        let mut sets_to_free: Vec<vk::DescriptorSet> = Vec::new();
        for (_entity, mut render_data) in self.text_render_resources.drain() {
            unsafe {
                allocator.destroy_buffer(render_data.transform_ubo, &mut render_data.transform_alloc);
                allocator.destroy_buffer(render_data.vertex_buffer, &mut render_data.vertex_alloc);
                if render_data.descriptor_set_0 != vk::DescriptorSet::null() {
                    sets_to_free.push(render_data.descriptor_set_0);
                }
            }
        }
        if !sets_to_free.is_empty() {
            unsafe {
                if let Err(e) = device.free_descriptor_sets(self.descriptor_pool, &sets_to_free) {
                    error!("[TextRenderer::cleanup] Failed to free cached text descriptor sets: {:?}", e);
                }
            }
        }
        info!("[TextRenderer::cleanup] Cleaned up cached text render resources.");
    }
}