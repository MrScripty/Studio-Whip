use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, MouseButton, Event};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;
use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use crate::gui_framework::context::vulkan_setup::{setup_vulkan, cleanup_vulkan}; // Ensure cleanup_vulkan is imported
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
        // --- ADDED LOGGING ---
        println!("[Handler::resumed] Start");
        // --- END LOGGING ---
        if self.vulkan_context.window.is_none() {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes().with_inner_size(PhysicalSize::new(600, 300))
            ).unwrap());
            // --- ADDED LOGGING ---
            println!("[Handler::resumed] Window created");
            // --- END LOGGING ---
            self.vulkan_context.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };
            setup_vulkan(&mut self.vulkan_context, window);
            // --- ADDED LOGGING ---
            println!("[Handler::resumed] Vulkan setup complete");
            // --- END LOGGING ---

            let renderer_instance = Renderer::new(&mut self.vulkan_context, extent, &self.scene.lock().unwrap());
            let renderer_arc = Arc::new(Mutex::new(renderer_instance));
            self.renderer = Some(renderer_arc.clone());
            // --- ADDED LOGGING ---
            println!("[Handler::resumed] Renderer created and stored");
            // --- END LOGGING ---

            let scene_handler = SceneEventHandler { scene_ref: self.scene.clone() };
            self.event_bus.subscribe_handler(scene_handler);

            self.event_bus.subscribe_arc(renderer_arc);
            // --- ADDED LOGGING ---
            println!("[Handler::resumed] Event handlers subscribed");
            // Request initial redraw after setup
            if let Some(window) = &self.vulkan_context.window {
                 println!("[Handler::resumed] Requesting initial redraw...");
                 window.request_redraw();
            }
            // --- END LOGGING ---
        }
        // --- ADDED LOGGING ---
        println!("[Handler::resumed] Finished");
        // --- END LOGGING ---
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");

                // --- RE-ADDED: CLEAR EVENT BUS SUBSCRIBERS ---
                println!("Clearing event bus subscribers...");
                self.event_bus.clear();
                // --- END RE-ADDED ---

                // --- RE-ADDED: EXPLICIT RENDERER CLEANUP ---
                if let Some(renderer_arc) = self.renderer.take() {
                    match Arc::try_unwrap(renderer_arc) {
                        Ok(renderer_mutex) => {
                            match renderer_mutex.into_inner() {
                                Ok(renderer) => {
                                    println!("Calling Renderer::cleanup...");
                                    renderer.cleanup(&mut self.vulkan_context);
                                    println!("Renderer::cleanup finished.");
                                }
                                Err(poisoned) => {
                                    eprintln!("Error: Renderer Mutex was poisoned during cleanup: {:?}", poisoned);
                                }
                            }
                        }
                        Err(failed_arc) => {
                            eprintln!("Error: Could not get exclusive ownership of Renderer Arc during cleanup (strong_count = {}). Renderer resources may leak!", Arc::strong_count(&failed_arc));
                        }
                    }
                } else {
                    println!("Renderer already taken or never initialized during close request.");
                }
                // --- END RE-ADDED ---


                // Proceed with Vulkan context cleanup
                println!("Calling cleanup_vulkan...");
                cleanup_vulkan(&mut self.vulkan_context);
                println!("cleanup_vulkan finished.");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // --- ADDED LOGGING ---
                println!("[Handler::window_event] RedrawRequested received");
                // --- END LOGGING ---
                if !self.resizing {
                    if let Some(renderer_arc) = &self.renderer {
                        // --- ADDED LOGGING ---
                        println!("[Handler::window_event] Calling renderer.render()...");
                        // --- END LOGGING ---
                        let mut renderer_guard = renderer_arc.lock().unwrap();
                        let scene_guard = self.scene.lock().unwrap();
                        renderer_guard.render(&mut self.vulkan_context, &scene_guard);
                        // --- ADDED LOGGING ---
                        println!("[Handler::window_event] renderer.render() finished.");
                        // --- END LOGGING ---
                    } else {
                         println!("[Handler::window_event] RedrawRequested skipped: Renderer is None.");
                    }
                    // Request redraw again for continuous rendering loop
                    if let Some(window) = &self.vulkan_context.window {
                        window.request_redraw();
                    }
                } else {
                     println!("[Handler::window_event] RedrawRequested skipped: Resizing.");
                }
            }
            WindowEvent::Resized(size) => {
                println!("[Handler::window_event] Resized to: {:?}", size); // Log resize
                self.resizing = true;
                if let Some(renderer_arc) = &mut self.renderer {
                    println!("[Handler::window_event] Calling renderer.resize_renderer()..."); // Log before resize
                    let mut renderer_guard = renderer_arc.lock().unwrap();
                    let mut scene_guard = self.scene.lock().unwrap();
                    renderer_guard.resize_renderer(&mut self.vulkan_context, &mut scene_guard, size.width, size.height);
                    println!("[Handler::window_event] renderer.resize_renderer() finished."); // Log after resize
                }
                self.resizing = false;
                // Request redraw after resize completes
                 if let Some(window) = &self.vulkan_context.window {
                     println!("[Handler::window_event] Requesting redraw after resize...");
                     window.request_redraw();
                 }
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