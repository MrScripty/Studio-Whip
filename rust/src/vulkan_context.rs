use ash::vk;
use ash::{Entry, Instance};
use ash::khr::{surface, swapchain};
use std::sync::Arc;
use vk_mem::Allocator;
use winit::window::Window;

pub struct VulkanContext {
    pub window: Option<Arc<Window>>,
    pub entry: Option<Entry>,
    pub instance: Option<Instance>,
    pub surface_loader: Option<surface::Instance>,
    pub surface: Option<vk::SurfaceKHR>,
    pub device: Option<ash::Device>,
    pub queue: Option<vk::Queue>,
    pub allocator: Option<Arc<Allocator>>,
    pub swapchain_loader: Option<swapchain::Device>,
    pub swapchain: Option<vk::SwapchainKHR>,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub vertex_buffer: Option<vk::Buffer>,
    pub vertex_allocation: Option<vk_mem::Allocation>,
    pub render_pass: Option<vk::RenderPass>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub vertex_shader: Option<vk::ShaderModule>,
    pub fragment_shader: Option<vk::ShaderModule>,
    pub pipeline_layout: Option<vk::PipelineLayout>,
    pub pipeline: Option<vk::Pipeline>,
    pub command_pool: Option<vk::CommandPool>,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphore: Option<vk::Semaphore>,
    pub render_finished_semaphore: Option<vk::Semaphore>,
    pub fence: Option<vk::Fence>,
    pub current_image: usize,
}

impl VulkanContext {
    pub fn new() -> Self {
        Self {
            window: None,
            entry: None,
            instance: None,
            surface_loader: None,
            surface: None,
            device: None,
            queue: None,
            allocator: None,
            swapchain_loader: None,
            swapchain: None,
            images: Vec::new(),
            image_views: Vec::new(),
            vertex_buffer: None,
            vertex_allocation: None,
            render_pass: None,
            framebuffers: Vec::new(),
            vertex_shader: None,
            fragment_shader: None,
            pipeline_layout: None,
            pipeline: None,
            command_pool: None,
            command_buffers: Vec::new(),
            image_available_semaphore: None,
            render_finished_semaphore: None,
            fence: None,
            current_image: 0,
        }
    }
}