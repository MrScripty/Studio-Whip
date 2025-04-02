use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, MouseButton, Event};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;
use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use crate::gui_framework::context::vulkan_setup::{setup_vulkan, cleanup_vulkan};
use crate::gui_framework::rendering::render_engine::Renderer;
use crate::gui_framework::interaction::controller::InteractionController;
use crate::gui_framework::event_bus::{EventBus, BusEvent as BusEvent, EventHandler};
use std::sync::{Arc, Mutex};

struct SceneEventHandler {
    scene_ref: Arc<Mutex<Scene>>,
}

impl EventHandler for SceneEventHandler {
    fn handle(&mut self, event: &BusEvent) {
        //println!("[EventHandler] Received event: {:?}", event); // <<< ADD LOG
        match event {
            BusEvent::ObjectMoved(index, delta, instance_id) => {
                let mut scene = self.scene_ref.lock().unwrap();
                // Call the existing public method on Scene
                scene.translate_object(*index, delta[0], delta[1], *instance_id);
            }
            // Handle other events Scene might care about later
            _ => {}
        }
    }

    // Implement as_any if downcasting is ever needed
    fn as_any(&self) -> &dyn std::any::Any { self }
}

pub struct VulkanContextHandler {
    vulkan_context: VulkanContext,
    scene: Arc<Mutex<Scene>>,
    renderer: Option<Renderer>,
    resizing: bool,
    controller: InteractionController,
    event_bus: Arc<EventBus>,
}

impl VulkanContextHandler {
    pub fn new(platform: VulkanContext, scene: Arc<Mutex<Scene>>, event_bus: Arc<EventBus>) -> Self {
        Self {
            vulkan_context: platform,
            scene,
            renderer: None,
            resizing: false,
            controller: InteractionController::new(),
            event_bus,
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
            self.renderer = Some(Renderer::new(&mut self.vulkan_context, extent, &self.scene.lock().unwrap()));

            let scene_handler = SceneEventHandler { scene_ref: self.scene.clone() };
            self.event_bus.subscribe(scene_handler);
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
                if !self.resizing {
                    if let Some(renderer) = &mut self.renderer {
                        // Pass locked scene to render
                        renderer.render(&mut self.vulkan_context, &self.scene.lock().unwrap());
                    }
                    if let Some(window) = &self.vulkan_context.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.resizing = true;
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize_renderer(&mut self.vulkan_context, &mut self.scene.lock().unwrap(), size.width, size.height);
                }
                self.resizing = false;
                if let Some(window) = &self.vulkan_context.window {
                }
            }
            WindowEvent::MouseInput { state: _state, button, .. } => {
                if button == MouseButton::Left {
                    let wrapped_event = Event::WindowEvent { event, window_id: _id };
                    // Lock the scene ONLY for the initial pick operation within the controller
                    let scene_guard = self.scene.lock().unwrap();
                    self.controller.handle_event(&wrapped_event, Some(&*scene_guard), None, self.vulkan_context.window.as_ref().unwrap(), &self.event_bus);
                    // scene_guard is dropped here, releasing the lock BEFORE the next event
                }
            }
            WindowEvent::CursorMoved { position: _position, .. } => {
                let wrapped_event = Event::WindowEvent { event, window_id: _id };
                // DO NOT lock the scene here. The controller doesn't need scene access during drag.
                // Pass None for the scene argument.
                self.controller.handle_event(&wrapped_event, None, None, self.vulkan_context.window.as_ref().unwrap(), &self.event_bus);
            }
            _ => (),
        }
    }
}