use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;
use std::sync::Arc;
use ash::vk;
use crate::vulkan_context::VulkanContext;
use crate::scene::Scene;
use crate::vulkan_setup::{setup_vulkan, cleanup_vulkan};
use crate::render_engine::Renderer;

pub struct VulkanContextHandler {
    vulkan_context: VulkanContext,
    scene: Scene,
    renderer: Option<Renderer>,
    resizing: bool,
}

impl VulkanContextHandler {
    pub fn new(platform: VulkanContext, scene: Scene) -> Self {
        Self {
            vulkan_context: platform,
            scene,
            renderer: None,
            resizing: false,
        }
    }
}

impl ApplicationHandler for VulkanContextHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.vulkan_context.window.is_none() {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes().with_inner_size(PhysicalSize::new(600, 300))
            ).unwrap());
            self.vulkan_context.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };
            setup_vulkan(&mut self.vulkan_context, window);
            self.renderer = Some(Renderer::new(&mut self.vulkan_context, extent, &self.scene));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                if let Some(renderer) = self.renderer.take() {
                    renderer.cleanup(&mut self.vulkan_context);
                }
                cleanup_vulkan(&mut self.vulkan_context);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if !self.resizing { // Skip rendering during resize to avoid mid-state issues
                    if let Some(renderer) = &self.renderer {
                        renderer.render(&mut self.vulkan_context);
                    }
                    if let Some(window) = &self.vulkan_context.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.resizing = true;
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize_renderer(&mut self.vulkan_context, size.width, size.height);
                }
                self.resizing = false;
                if let Some(window) = &self.vulkan_context.window {
                    window.request_redraw(); // Trigger redraw after resize
                }
            }
            _ => (),
        }
    }
}