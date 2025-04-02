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
use crate::gui_framework::event_bus::{EventBus, BusEvent, EventHandler};
use std::sync::{Arc, Mutex};
// Removed unused Any import

struct SceneEventHandler {
    scene_ref: Arc<Mutex<Scene>>,
}

impl EventHandler for SceneEventHandler {
    fn handle(&mut self, event: &BusEvent) {
        match event {
            BusEvent::ObjectMoved(index, delta, instance_id) => {
                let mut scene = self.scene_ref.lock().unwrap();
                scene.translate_object(*index, delta[0], delta[1], *instance_id);
            }
            _ => {}
        }
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

pub struct VulkanContextHandler {
    vulkan_context: VulkanContext,
    scene: Arc<Mutex<Scene>>,
    renderer: Option<Arc<Mutex<Renderer>>>,
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

            let renderer_instance = Renderer::new(&mut self.vulkan_context, extent, &self.scene.lock().unwrap());
            let renderer_arc = Arc::new(Mutex::new(renderer_instance));
            self.renderer = Some(renderer_arc.clone());

            let scene_handler = SceneEventHandler { scene_ref: self.scene.clone() };
            self.event_bus.subscribe_handler(scene_handler);

            self.event_bus.subscribe_arc(renderer_arc);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                let _ = self.renderer.take();
                cleanup_vulkan(&mut self.vulkan_context);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if !self.resizing {
                    if let Some(renderer_arc) = &self.renderer {
                        let mut renderer_guard = renderer_arc.lock().unwrap();
                        let scene_guard = self.scene.lock().unwrap();
                        renderer_guard.render(&mut self.vulkan_context, &scene_guard);
                    }
                    if let Some(window) = &self.vulkan_context.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.resizing = true;
                if let Some(renderer_arc) = &mut self.renderer {
                    let mut renderer_guard = renderer_arc.lock().unwrap();
                    let mut scene_guard = self.scene.lock().unwrap();
                    renderer_guard.resize_renderer(&mut self.vulkan_context, &mut scene_guard, size.width, size.height);
                }
                self.resizing = false;
            }
            WindowEvent::MouseInput { state: _state, button, .. } => {
                if button == MouseButton::Left {
                    let wrapped_event = Event::WindowEvent { event, window_id: _id };
                    let scene_guard = self.scene.lock().unwrap();
                    self.controller.handle_event(&wrapped_event, Some(&*scene_guard), None, self.vulkan_context.window.as_ref().unwrap(), &self.event_bus);
                }
            }
            WindowEvent::CursorMoved { position: _position, .. } => {
                let wrapped_event = Event::WindowEvent { event, window_id: _id };
                self.controller.handle_event(&wrapped_event, None, None, self.vulkan_context.window.as_ref().unwrap(), &self.event_bus);
            }
            _ => (),
        }
    }
}