// /mnt/c/Users/jerem/Desktop/Studio-Whip/rust/src/gui_framework/window/window_handler.rs

use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, Event};
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
use std::any::Any;

// Handles events related to Scene state changes (like ObjectMoved)
struct SceneEventHandler {
    scene_ref: Arc<Mutex<Scene>>,
}

impl EventHandler for SceneEventHandler {
    fn handle(&mut self, event: &BusEvent) {
        match event {
            BusEvent::ObjectMoved(index, delta, instance_id) => {
                if let Ok(mut scene) = self.scene_ref.lock() {
                    scene.translate_object(*index, delta[0], delta[1], *instance_id);
                } else {
                    eprintln!("[SceneEventHandler] Warning: Could not lock scene mutex for ObjectMoved.");
                }
            }
            // Handle FieldUpdated for visibility if implemented in Task 3
            // BusEvent::FieldUpdated { object_id, field_name, value } => {
            //     if field_name == "visible" {
            //         if let Ok(mut scene) = self.scene_ref.lock() {
            //             if let Some(obj) = scene.pool.get_mut(*object_id) { // Assuming get_mut or direct access
            //                 if let Some(visible_val) = value.downcast_ref::<bool>() {
            //                     obj.visible = *visible_val;
            //                 }
            //             }
            //         }
            //     }
            // }
            _ => {} // Ignore other events like InstanceAdded, ObjectPicked, HotkeyPressed
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

// Main application handler managing the event loop and core components
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
            controller: InteractionController::new(), // Controller loads hotkeys here
            event_bus,
        }
    }
}

impl ApplicationHandler for VulkanContextHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("[Handler::resumed] Start");
        if self.vulkan_context.window.is_none() {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes().with_inner_size(PhysicalSize::new(600, 300))
            ).unwrap());
            println!("[Handler::resumed] Window created");
            self.vulkan_context.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };
            setup_vulkan(&mut self.vulkan_context, window);
            println!("[Handler::resumed] Vulkan setup complete");

            // Create Renderer (requires locking scene)
            let renderer_instance = { // Scope the scene lock
                let scene_guard = self.scene.lock().expect("Failed to lock scene for renderer creation");
                Renderer::new(&mut self.vulkan_context, extent, &scene_guard)
            };
            let renderer_arc = Arc::new(Mutex::new(renderer_instance));
            self.renderer = Some(renderer_arc.clone());
            println!("[Handler::resumed] Renderer created and stored");

            // Subscribe SceneEventHandler
            let scene_handler = SceneEventHandler { scene_ref: self.scene.clone() };
            self.event_bus.subscribe_handler(scene_handler);

            // Subscribe Renderer (as EventHandler for InstanceAdded)
            self.event_bus.subscribe_arc(renderer_arc);
            println!("[Handler::resumed] Event handlers subscribed");

            // Request initial redraw
            if let Some(window) = &self.vulkan_context.window {
                 println!("[Handler::resumed] Requesting initial redraw...");
                 window.request_redraw();
            }
        }
        println!("[Handler::resumed] Finished");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {

        // --- Pass relevant events to InteractionController ---
        // Controller handles MouseInput, CursorMoved, KeyboardInput, ModifiersChanged
        // It publishes BusEvents (ObjectPicked, ObjectMoved, HotkeyPressed)
        match &event {
             WindowEvent::MouseInput { .. }
             | WindowEvent::CursorMoved { .. }
             | WindowEvent::KeyboardInput { .. }
             | WindowEvent::ModifiersChanged { .. } => {
                 let wrapped_event = Event::WindowEvent { event: event.clone(), window_id: id };

                 // Lock scene only when needed (MouseInput for picking)
                 let scene_guard = if matches!(wrapped_event, Event::WindowEvent{ event: WindowEvent::MouseInput{ state: winit::event::ElementState::Pressed, ..}, ..}) {
                     // Lock scene only on mouse press for picking
                     Some(self.scene.lock().expect("Failed to lock scene for picking"))
                 } else {
                     None
                 };

                 // Get window reference (assuming it exists)
                 let window_ref = self.vulkan_context.window.as_ref().expect("Window not available for event handling");

                 self.controller.handle_event(
                     &wrapped_event,
                     scene_guard.as_deref(), // Pass Option<&Scene>
                     None, // Renderer not needed by controller
                     window_ref,
                     &self.event_bus
                 );

                 // If the controller handled the event (e.g., published Escape hotkey),
                 // we might not need to process it further in the match below.
                 // However, CloseRequested needs to be handled here regardless of source.
             }
             _ => {} // Process other events below
        }

        // --- Handle core window events ---
        match event {
            WindowEvent::CloseRequested => {
                println!("CloseRequested event received (Window X button or Hotkey)");

                // Cleanup sequence:
                // 1. Clear event bus subscribers to prevent further event processing during shutdown
                println!("Clearing event bus subscribers...");
                self.event_bus.clear();

                // 2. Cleanup Renderer (waits for GPU idle, destroys Vulkan objects)
                if let Some(renderer_arc) = self.renderer.take() {
                    // Use try_unwrap to ensure we have exclusive ownership before destroying
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
                                    // Resources might leak, but proceed with Vulkan cleanup
                                }
                            }
                        }
                        Err(failed_arc) => {
                            // This indicates another Arc reference exists, which shouldn't happen
                            // if cleanup is ordered correctly. Log error, resources will leak.
                            eprintln!("Error: Could not get exclusive ownership of Renderer Arc during cleanup (strong_count = {}). Renderer resources may leak!", Arc::strong_count(&failed_arc));
                            // Attempt to drop the Arc anyway
                            drop(failed_arc);
                        }
                    }
                } else {
                    println!("Renderer already taken or never initialized during close request.");
                }

                // 3. Cleanup core Vulkan context (device, instance, etc.)
                println!("Calling cleanup_vulkan...");
                cleanup_vulkan(&mut self.vulkan_context);
                println!("cleanup_vulkan finished.");

                // 4. Exit the event loop
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if !self.resizing {
                    if let Some(renderer_arc) = &self.renderer {
                        // Lock renderer and scene for rendering
                        match (renderer_arc.lock(), self.scene.lock()) {
                            (Ok(mut renderer_guard), Ok(scene_guard)) => {
                                println!("[Handler::window_event] Calling renderer.render()...");
                                renderer_guard.render(&mut self.vulkan_context, &scene_guard);
                                println!("[Handler::window_event] renderer.render() finished.");
                            }
                            (Err(_), _) => eprintln!("[Handler::window_event] Error: Could not lock Renderer mutex for redraw."),
                            (_, Err(_)) => eprintln!("[Handler::window_event] Error: Could not lock Scene mutex for redraw."),
                        }
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
                println!("[Handler::window_event] Resized to: {:?}", size);
                if size.width > 0 && size.height > 0 {
                    self.resizing = true;
                    if let Some(renderer_arc) = &mut self.renderer {
                         match (renderer_arc.lock(), self.scene.lock()) {
                            (Ok(mut renderer_guard), Ok(mut scene_guard)) => {
                                println!("[Handler::window_event] Calling renderer.resize_renderer()...");
                                renderer_guard.resize_renderer(&mut self.vulkan_context, &mut scene_guard, size.width, size.height);
                                println!("[Handler::window_event] renderer.resize_renderer() finished.");
                            }
                            (Err(_), _) => eprintln!("[Handler::window_event] Error: Could not lock Renderer mutex for resize."),
                            (_, Err(_)) => eprintln!("[Handler::window_event] Error: Could not lock Scene mutex for resize."),
                        }
                    }
                    self.resizing = false;
                    // Request redraw after resize completes
                     if let Some(window) = &self.vulkan_context.window {
                         println!("[Handler::window_event] Requesting redraw after resize...");
                         window.request_redraw();
                     }
                } else {
                    println!("[Handler::window_event] Resized to zero dimensions, skipping resize logic.");
                }
            }
            // Other events (MouseInput, CursorMoved, KeyboardInput, ModifiersChanged)
            // are passed to the controller at the beginning of the function.
            _ => (),
        }
    }

    // Note: Handling of BusEvent::HotkeyPressed is done via a subscriber
    // (e.g., HotkeyActionHandler in main.rs) listening on the event_bus.
    // The controller publishes the event, and the handler reacts.
    // If HotkeyPressed needs to directly interact with the event loop (e.g., exit),
    // the handler would need access to an EventLoopProxy or the action needs
    // to trigger a standard event like CloseRequested.
}