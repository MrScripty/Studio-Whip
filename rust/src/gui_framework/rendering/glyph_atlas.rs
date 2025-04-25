use crate::gui_framework::context::vulkan_context::VulkanContext;
use ash::vk;
use bevy_ecs::system::Resource;
use bevy_log::{info, error};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vk_mem::Allocation;
use vk_mem::Alloc;

// Represents the location and UV coordinates of a single glyph within the atlas
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    // Store pixel coordinates directly later when packing is implemented
    pub pixel_x: u32,
    pub pixel_y: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub uv_min: [f32; 2], // Top-left UV coordinate
    pub uv_max: [f32; 2], // Bottom-right UV coordinate
}

// Manages the Vulkan texture atlas for glyphs
#[derive(Debug)]
pub struct GlyphAtlas {
    pub image: vk::Image,
    pub allocation: Option<Allocation>, 
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub extent: vk::Extent2D,
    pub format: vk::Format,
    // We won't use a packer struct directly
    glyph_cache: HashMap<u64, GlyphInfo>, // Maps glyph key (e.g., cosmic_text GlyphId) to its info
    // We'll need the allocator later for uploads, store it or pass it in methods
}

impl GlyphAtlas {
    pub fn new(vk_context: &mut VulkanContext, initial_extent: vk::Extent2D) -> Result<Self, String> {
        info!("[GlyphAtlas::new] Creating glyph atlas with extent: {:?}", initial_extent);
        let device = vk_context.device.as_ref().ok_or("Device not available")?.clone(); // Clone Arc
        let allocator = vk_context.allocator.as_ref().ok_or("Allocator not available")?.clone(); // Clone Arc

        let format = vk::Format::R8_UNORM; // Grayscale, 8-bit unsigned normalized (common for alpha masks)

        // --- Create Vulkan Image ---
        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width: initial_extent.width,
                height: initial_extent.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            // TRANSFER_DST: To copy rasterized glyphs into it
            // SAMPLED: To be read by shaders
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED, // Will transition layout before use
            ..Default::default()
        };

        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice, // Let VMA decide, likely GPU local
            ..Default::default()
        };

        let (image, allocation) = unsafe {
            allocator.create_image(&image_create_info, &allocation_create_info)
        }.map_err(|e| format!("Failed to create glyph atlas image: {:?}", e))?;
        info!("[GlyphAtlas::new] Vulkan image created.");

        // --- Create Image View ---
        let image_view_create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        let image_view = unsafe {
            device.create_image_view(&image_view_create_info, None)
        }.map_err(|e| format!("Failed to create glyph atlas image view: {:?}", e))?;
        info!("[GlyphAtlas::new] Vulkan image view created.");

        // --- Create Sampler ---
        let sampler_create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR, // Linear filtering for smoother scaling
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::FALSE, // Basic sampler
            max_anisotropy: 1.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 1.0, // Only one mip level
            border_color: vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            unnormalized_coordinates: vk::FALSE, // Use normalized UVs [0, 1]
            ..Default::default()
        };
        let sampler = unsafe {
            device.create_sampler(&sampler_create_info, None)
        }.map_err(|e| format!("Failed to create glyph atlas sampler: {:?}", e))?;
        info!("[GlyphAtlas::new] Vulkan sampler created.");

        // No packer struct to initialize

        Ok(Self {
            image,
            allocation: Some(allocation),
            image_view,
            sampler,
            extent: initial_extent,
            format,
            glyph_cache: HashMap::new(),
        })
    }

    // Placeholder for adding glyphs - will involve rasterization and upload
    pub fn add_glyph(&mut self, /* glyph_key: u64, glyph_bitmap: &[u8], width: u32, height: u32 */) -> Result<&GlyphInfo, String> {
        // 1. Check cache
        // 2. If not present:
        //    a. Use rectangle_pack::pack_rects function with TargetBin and RectToInsert
        //    b. If space found (RectanglePackOk):
        //       i. Rasterize/get bitmap data (passed in for now)
        //       ii. Upload bitmap to self.image at the PackedLocation (using staging buffer)
        //       iii. Calculate UVs based on PackedLocation and self.extent
        //       iv. Store GlyphInfo (with pixel coords and UVs) in self.glyph_cache
        //       v. Return reference to cached info
        //    c. If no space (RectanglePackError):
        //       i. Return error (or handle atlas resizing - complex)
        // 3. If present, return reference to cached info
        error!("[GlyphAtlas::add_glyph] Not yet implemented!");
        Err("add_glyph not implemented".to_string())
    }

    // Cleanup Vulkan resources
    pub fn cleanup(&mut self, vk_context: &VulkanContext) {
        info!("[GlyphAtlas::cleanup] Cleaning up glyph atlas resources...");
        let Some(device) = vk_context.device.as_ref() else {
            error!("[GlyphAtlas::cleanup] Device not available for cleanup.");
            return;
        };
        let Some(allocator) = vk_context.allocator.as_ref() else {
            error!("[GlyphAtlas::cleanup] Allocator not available for cleanup.");
            // Need allocator to destroy image+allocation safely
            return;
        };

        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.image_view, None);
            // Take ownership of allocation before destroying image
            if let Some(mut alloc) = self.allocation.take() {
                allocator.destroy_image(self.image, &mut alloc); // Pass &mut alloc
            } else {
                error!("[GlyphAtlas::cleanup] Allocation was already taken or None!");
                // Maybe destroy image anyway? Or just log?
                // device.destroy_image(self.image, None); // Potentially unsafe without allocator
            }
        }
        info!("[GlyphAtlas::cleanup] Finished.");
    }
}

// --- Bevy Resource ---

// Using Arc<Mutex> for interior mutability, similar to VulkanContextResource
#[derive(Resource, Clone)]
pub struct GlyphAtlasResource(pub Arc<Mutex<GlyphAtlas>>);