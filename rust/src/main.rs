// /mnt/c/Users/jerem/Desktop/Studio-Whip/rust/src/main.rs
use winit::event_loop::{EventLoop, ControlFlow, EventLoopProxy};
use winit::event::{Event, WindowEvent, StartCause, ElementState};
use winit::window::Window;
use rusty_whip::gui_framework::{VulkanContext, Scene, RenderObject, Renderer, EventBus, BusEvent, EventHandler, InteractionController};
use rusty_whip::Vertex;
use rusty_whip::gui_framework::context::vulkan_setup::{setup_vulkan, cleanup_vulkan};
use std::sync::{Arc, Mutex};
use std::any::Any;
use ash::vk;
use std::collections::HashMap;

// Define a custom event type
#[derive(Debug, Clone, PartialEq, Eq)]
enum UserEvent {
    Exit,
}

// Hotkey Handler using Proxy
struct HotkeyActionHandler {
    proxy: EventLoopProxy<UserEvent>,
}
impl EventHandler for HotkeyActionHandler {
    fn handle(&mut self, event: &BusEvent) {
        if let BusEvent::HotkeyPressed(Some(action)) = event {
            println!("[Main] Hotkey Action Received: {}", action);
            if action == "CloseRequested" {
                println!("[Main] CloseRequested action received, sending UserEvent::Exit.");
                if let Err(e) = self.proxy.send_event(UserEvent::Exit) {
                     eprintln!("[Main] Error sending Exit event: {}", e);
                }
            }
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}


// Scene Event Handler
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
             _ => {}
         }
     }
     fn as_any(&self) -> &dyn Any { self }
}

// Click Router Definition
struct ClickRouter {
    // Stores object_id -> Mutex-protected callback function
    // The closure must be Send + 'static to be stored across threads safely within the Mutex.
    callbacks: HashMap<usize, Box<Mutex<dyn FnMut(usize, Option<usize>) + Send + 'static>>>,
}

impl ClickRouter {
    fn new() -> Self {
        ClickRouter {
            callbacks: HashMap::new(),
        }
    }

    // API for registering a handler
    pub fn register_click_handler(
        &mut self,
        object_id: usize,
        // Require Send + 'static for the closure itself
        callback: impl FnMut(usize, Option<usize>) + Send + 'static,
    ) {
        println!("[ClickRouter] Registering handler for object ID: {}", object_id);
        // Wrap the callback in a Mutex before boxing
        self.callbacks.insert(object_id, Box::new(Mutex::new(callback)));
    }
}

// Implement EventHandler for ClickRouter
impl EventHandler for ClickRouter {
    fn handle(&mut self, event: &BusEvent) {
        if let BusEvent::ObjectClicked(clicked_id, instance_id) = event {
            println!("[ClickRouter] Received ObjectClicked event for ID: {}", clicked_id);
            // Get the Box<Mutex<...>> - no need for get_mut here
            if let Some(callback_mutex_box) = self.callbacks.get(clicked_id) {
                println!("[ClickRouter] Found handler for ID: {}. Locking and executing...", clicked_id);
                // Lock the mutex for this specific callback
                match callback_mutex_box.lock() {
                    Ok(mut callback_guard) => {
                        // Execute the callback using the guard
                        (*callback_guard)(*clicked_id, *instance_id);
                    }
                    Err(e) => {
                        eprintln!("[ClickRouter] Error: Could not lock callback mutex for ID {}: {}", clicked_id, e);
                    }
                }
            } else {
                 println!("[ClickRouter] No handler registered for ID: {}", clicked_id);
            }
        }
    }

    fn as_any(&self) -> &dyn Any { self }
}


fn main() {
    let event_loop = EventLoop::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let proxy = event_loop.create_proxy();

    let mut vulkan_context_option: Option<VulkanContext> = Some(VulkanContext::new());
    let event_bus = Arc::new(EventBus::new());
    let scene = Arc::new(Mutex::new(Scene::new(event_bus.clone())));
    let mut renderer_option: Option<Arc<Mutex<Renderer>>> = None;
    let mut controller_option: Option<InteractionController> = None;
    let mut window_option: Option<Arc<Window>> = None;
    let mut resizing = false;

    let click_router = Arc::new(Mutex::new(ClickRouter::new()));

    let hotkey_handler = HotkeyActionHandler { proxy: proxy.clone() };
    event_bus.subscribe_handler(hotkey_handler);

    let scene_event_handler = SceneEventHandler { scene_ref: scene.clone() };
    event_bus.subscribe_handler(scene_event_handler);

    event_bus.subscribe_arc(click_router.clone());

    let mut triangle_id_opt: Option<usize> = None;
    let mut square_id_opt: Option<usize> = None;

    {
        let mut scene_guard = scene.lock().unwrap();
        let width = 600.0;
        let height = 300.0;
        scene_guard.add_object(RenderObject {
            vertices: vec![ Vertex { position: [0.0, 0.0] }, Vertex { position: [0.0, height] }, Vertex { position: [width, height] }, Vertex { position: [width, 0.0] }, ],
            vertex_shader_filename: "background.vert.spv".to_string(),
            fragment_shader_filename: "background.frag.spv".to_string(),
            depth: 0.0,
            on_window_resize_scale: true,
            on_window_resize_move: false,
            offset: [0.0, 0.0],
            is_draggable: false,
            instances: Vec::new(),
            visible: true,
        });
        let triangle_id = scene_guard.add_object(RenderObject {
            vertices: vec![ Vertex { position: [275.0, 125.0] }, Vertex { position: [300.0, 175.0] }, Vertex { position: [325.0, 125.0] }, ],
            vertex_shader_filename: "triangle.vert.spv".to_string(),
            fragment_shader_filename: "triangle.frag.spv".to_string(),
            depth: 1.0,
            on_window_resize_scale: false,
            on_window_resize_move: true,
            offset: [0.0, 0.0],
            is_draggable: true,
            instances: Vec::new(),
            visible: true,
        });
        triangle_id_opt = Some(triangle_id);
        let square_id = scene_guard.add_object(RenderObject {
            vertices: vec![ Vertex { position: [100.0, 50.0] }, Vertex { position: [100.0, 100.0] }, Vertex { position: [150.0, 100.0] }, Vertex { position: [150.0, 50.0] }, ],
            vertex_shader_filename: "square.vert.spv".to_string(),
            fragment_shader_filename: "square.frag.spv".to_string(),
            depth: 2.0,
            on_window_resize_scale: false,
            on_window_resize_move: true,
            offset: [0.0, 0.0],
            is_draggable: true,
            instances: Vec::new(),
            visible: true,
        });
        square_id_opt = Some(square_id);
         let _small_square_id = scene_guard.add_object(RenderObject {
             vertices: vec![ Vertex { position: [400.0, 200.0] }, Vertex { position: [400.0, 230.0] }, Vertex { position: [430.0, 230.0] }, Vertex { position: [430.0, 200.0] }, ],
             vertex_shader_filename: "square.vert.spv".to_string(),
             fragment_shader_filename: "square.frag.spv".to_string(),
             depth: 3.0,
             on_window_resize_scale: false,
             on_window_resize_move: true,
             offset: [0.0, 0.0],
             is_draggable: true,
             instances: Vec::new(),
             visible: true,
         });
         let _vertical_rect_id = scene_guard.add_object(RenderObject {
             vertices: vec![ Vertex { position: [450.0, 190.0] }, Vertex { position: [450.0, 240.0] }, Vertex { position: [470.0, 240.0] }, Vertex { position: [470.0, 190.0] }, ],
             vertex_shader_filename: "square.vert.spv".to_string(),
             fragment_shader_filename: "square.frag.spv".to_string(),
             depth: 4.0,
             on_window_resize_scale: false,
             on_window_resize_move: true,
             offset: [0.0, 0.0],
             is_draggable: true,
             instances: Vec::new(),
             visible: false,
         });
        scene_guard.add_instance(triangle_id, [50.0, 50.0]);
        scene_guard.add_instance(triangle_id, [-50.0, -50.0]);
        scene_guard.add_instance(square_id, [100.0, 0.0]);

        scene_guard.groups().add_group("another_group").unwrap();
        { let mut group = scene_guard.groups().group("another_group").unwrap(); group.add_object(1); group.add_object(3); }

    }

    // Object Clicked Events
    {
        let mut router_guard = click_router.lock().expect("Failed to lock ClickRouter");

        if let Some(triangle_id) = triangle_id_opt {
            // Example of a closure that captures state (needs Send)
            let mut click_count = 0;
            router_guard.register_click_handler(triangle_id, move |id, instance_id| {
                click_count += 1; // Mutate captured state
                println!("---> Triangle (ID: {}) clicked! Instance: {:?}. Count: {}", id, instance_id, click_count);
            });
        }

        if let Some(square_id) = square_id_opt {
             // Simple non-capturing closure (also Send + 'static)
             router_guard.register_click_handler(square_id, |id, instance_id| {
                println!("---> Square (ID: {}) clicked! Instance: {:?}", id, instance_id);
            });
        }
    }


    println!("[Main] Starting event loop...");
    let _ = event_loop.run(move |event, elwt| {

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::Resumed => {
                if window_option.is_none() {
                    println!("[EventLoop] Resumed: Creating window and setting up Vulkan...");
                    let window = Arc::new(elwt.create_window(
                        Window::default_attributes().with_inner_size(winit::dpi::PhysicalSize::new(600, 300))
                    ).unwrap());
                    window_option = Some(window.clone());

                    let mut vk_ctx = vulkan_context_option.take().expect("VulkanContext missing on resume");
                    setup_vulkan(&mut vk_ctx, window.clone());

                    let extent = { let size = window.inner_size(); vk::Extent2D { width: size.width, height: size.height } };

                    let renderer_instance = {
                        let scene_guard = scene.lock().expect("Failed to lock scene for renderer creation");
                        Renderer::new(&mut vk_ctx, extent, &scene_guard)
                    };
                    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
                    renderer_option = Some(renderer_arc.clone());
                    controller_option = Some(InteractionController::new());

                    event_bus.subscribe_arc(renderer_arc);
                    println!("[EventLoop] Renderer and Controller created, handlers subscribed.");
                    vulkan_context_option = Some(vk_ctx);
                    window.request_redraw();
                }
            }

            Event::WindowEvent { window_id, event } => {
                 if window_option.as_ref().map_or(false, |win| win.id() == window_id) {
                    let window = window_option.as_ref().unwrap();

                    if let Some(controller) = controller_option.as_mut() {
                         match &event {
                             WindowEvent::MouseInput { .. } | WindowEvent::CursorMoved { .. } |
                             WindowEvent::KeyboardInput { .. } | WindowEvent::ModifiersChanged { .. } => {
                                 let wrapped_event = Event::WindowEvent { event: event.clone(), window_id };
                                 let scene_guard = if matches!(event, WindowEvent::MouseInput{ state: ElementState::Pressed, button: winit::event::MouseButton::Left, ..}) {
                                     Some(scene.lock().expect("Failed to lock scene for picking"))
                                 } else { None };
                                 controller.handle_event(&wrapped_event, scene_guard.as_deref(), None, window, &event_bus);
                             }
                             _ => {}
                         }
                    }

                    match event {
                        WindowEvent::CloseRequested => {
                            println!("[EventLoop] CloseRequested event received (Window X button)");
                            if let Err(e) = proxy.send_event(UserEvent::Exit) {
                                eprintln!("[EventLoop] Error sending Exit event on CloseRequested: {}", e);
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            if !resizing {
                                if let (Some(renderer_arc), Some(vk_ctx)) = (renderer_option.as_ref(), vulkan_context_option.as_mut()) {
                                    match (renderer_arc.lock(), scene.lock()) {
                                        (Ok(mut renderer_guard), Ok(scene_guard)) => {
                                            renderer_guard.render(vk_ctx, &scene_guard);
                                        }
                                        (Err(_), _) => eprintln!("[EventLoop] Error: Could not lock Renderer mutex for redraw."),
                                        (_, Err(_)) => eprintln!("[EventLoop] Error: Could not lock Scene mutex for redraw."),
                                    }
                                }
                            }
                        }
                        WindowEvent::Resized(size) => {
                            println!("[EventLoop] Resized to: {:?}", size);
                            if size.width > 0 && size.height > 0 {
                                resizing = true;
                                if let (Some(renderer_arc), Some(vk_ctx)) = (renderer_option.as_mut(), vulkan_context_option.as_mut()) {
                                     match (renderer_arc.lock(), scene.lock()) {
                                        (Ok(mut renderer_guard), Ok(mut scene_guard)) => {
                                            println!("[EventLoop] Calling renderer.resize_renderer()...");
                                            renderer_guard.resize_renderer(vk_ctx, &mut scene_guard, size.width, size.height);
                                            println!("[EventLoop] renderer.resize_renderer() finished.");
                                        }
                                        (Err(_), _) => eprintln!("[EventLoop] Error: Could not lock Renderer mutex for resize."),
                                        (_, Err(_)) => eprintln!("[EventLoop] Error: Could not lock Scene mutex for resize."),
                                    }
                                }
                                resizing = false;
                                window.request_redraw();
                            }
                        }
                        _ => {}
                    }
                 }
            }

            Event::UserEvent(user_event) => {
                println!("[EventLoop] UserEvent received: {:?}", user_event);
                match user_event {
                    UserEvent::Exit => {
                        println!("[EventLoop] Exit requested via UserEvent.");
                        elwt.exit();
                    }
                }
            }
            Event::LoopExiting => {
                println!("[EventLoop] LoopExiting: Cleaning up...");
                event_bus.clear();
                if let Some(renderer_arc) = renderer_option.take() {
                     match Arc::try_unwrap(renderer_arc) {
                         Ok(renderer_mutex) => {
                             match renderer_mutex.into_inner() {
                                 Ok(renderer) => {
                                     if let Some(vk_ctx) = vulkan_context_option.as_mut() {
                                         println!("[EventLoop] Calling Renderer::cleanup...");
                                         renderer.cleanup(vk_ctx);
                                         println!("[EventLoop] Renderer::cleanup finished.");
                                     } else { eprintln!("[EventLoop] Error: VulkanContext missing during Renderer cleanup."); }
                                 }
                                 Err(p) => eprintln!("Error: Renderer Mutex poisoned during cleanup: {:?}", p),
                             }
                         }
                         Err(arc) => eprintln!("Error: Could not get exclusive Renderer Arc during cleanup (refs: {}). Leaking!", Arc::strong_count(&arc)),
                     }
                }
                if let Some(mut vk_ctx) = vulkan_context_option.take() {
                    println!("[EventLoop] Calling cleanup_vulkan...");
                    cleanup_vulkan(&mut vk_ctx);
                    println!("[EventLoop] cleanup_vulkan finished.");
                }
                window_option = None;
                println!("[EventLoop] Cleanup complete.");
            }
            Event::AboutToWait => {
                 if let Some(window) = &window_option {
                     window.request_redraw();
                 }
            }
            _ => {}
        }
    });
}