use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use std::sync::Arc;
use ash::vk;
use crate::platform::Platform; // Renamed from App
use crate::scene::Scene;
use crate::vulkan_core::{setup_vulkan, cleanup_vulkan};
use crate::renderer::{setup_renderer, cleanup_renderer, render};

pub struct PlatformHandler {
    platform: Platform,
    scene: Scene,
}

impl PlatformHandler {
    pub fn new(platform: Platform, scene: Scene) -> Self {
        Self { platform, scene }
    }
}

impl ApplicationHandler for PlatformHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.platform.window.is_none() {
            let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
            self.platform.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };
            setup_vulkan(&mut self.platform, window);
            setup_renderer(&mut self.platform, extent, &self.scene);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                cleanup_renderer(&mut self.platform);
                cleanup_vulkan(&mut self.platform);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                render(&mut self.platform);
                if let Some(window) = &self.platform.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}