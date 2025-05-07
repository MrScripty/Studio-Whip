use ash::vk;
use ash::{Entry, Instance};
use ash::khr::{surface, swapchain};
use ash::ext::debug_utils;
use std::sync::Arc;
use vk_mem::Allocator;


pub struct VulkanContext {
    pub entry: Option<Entry>,
    pub instance: Option<Instance>,
    pub surface_loader: Option<surface::Instance>,
    pub surface: Option<vk::SurfaceKHR>,
    pub device: Option<ash::Device>,
    pub physical_device: Option<vk::PhysicalDevice>,
    pub queue: Option<vk::Queue>,
    pub queue_family_index: Option<u32>,
    pub allocator: Option<Arc<Allocator>>,
    pub swapchain_loader: Option<swapchain::Device>,
    pub swapchain: Option<vk::SwapchainKHR>,
    pub current_swap_extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    // --- Depth Buffer Resources ---
    pub depth_image: Option<vk::Image>,
    pub depth_image_allocation: Option<vk_mem::Allocation>,
    pub depth_image_view: Option<vk::ImageView>,
    pub depth_format: Option<vk::Format>,
    // --- End Depth Buffer ---
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_allocation: Option<vk_mem::Allocation>,
    pub render_pass: Option<vk::RenderPass>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub shape_pipeline_layout: Option<vk::PipelineLayout>,
    pub text_pipeline_layout: Option<vk::PipelineLayout>,
    pub command_pool: Option<vk::CommandPool>,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphore: Option<vk::Semaphore>,
    pub render_finished_semaphore: Option<vk::Semaphore>,
    pub fence: Option<vk::Fence>,
    pub current_image: usize,
    // --- Debug Messenger Fields ---
    pub debug_utils_loader: Option<debug_utils::Instance>,
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    pub debug_utils_device: Option<debug_utils::Device>,
}

impl VulkanContext {
    pub fn new() -> Self {
        Self {
            entry: None,
            instance: None,
            surface_loader: None,
            surface: None,
            device: None,
            physical_device: None, 
            queue: None,
            queue_family_index: None,
            allocator: None,
            swapchain_loader: None,
            swapchain: None,
            current_swap_extent: vk::Extent2D { width: 0, height: 0 },
            images: Vec::new(),
            image_views: Vec::new(),
            // --- Depth Buffer Resources ---
            depth_image: None,
            depth_image_allocation: None,
            depth_image_view: None,
            depth_format: None,
            // --- End Depth Buffer ---
            vertex_buffer: None,
            vertex_allocation: None,
            render_pass: None,
            framebuffers: Vec::new(),
            shape_pipeline_layout: None,
            text_pipeline_layout: None,
            command_pool: None,
            command_buffers: Vec::new(),
            image_available_semaphore: None,
            render_finished_semaphore: None,
            fence: None,
            current_image: 0,
            // --- Debug Messenger Fields ---
            debug_utils_loader: None,
            debug_messenger: None,
            debug_utils_device: None,
        }
    }
}