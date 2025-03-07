use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use std::sync::Arc;
use ash::vk;
use crate::application::App;
use crate::vulkan_core::{setup_vulkan, cleanup_vulkan};
use crate::renderer::{setup_renderer, cleanup_renderer, render};

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
            self.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };

            setup_vulkan(self, window);
            setup_renderer(self, extent);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
                cleanup_renderer(self);
                cleanup_vulkan(self);
            }
            WindowEvent::RedrawRequested => {
                render(self);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}