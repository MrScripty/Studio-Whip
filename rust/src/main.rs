// /mnt/c/Users/jerem/Desktop/Studio-Whip/rust/src/main.rs
use bevy_app::{App, AppExit, Startup, Update, Last};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::on_event;
use bevy_log::{info, error, warn, LogPlugin, Level};
use bevy_utils::default;
use bevy_input::keyboard::KeyboardFocusLost;
use bevy_input::InputPlugin;
use bevy_window::{
    PrimaryWindow, Window, WindowPlugin, WindowCloseRequested, PresentMode,
    WindowResolution,
};
use bevy_winit::{WinitPlugin, WinitWindows, WakeUp}; // Use WinitWindows to access raw winit::Window
use bevy_a11y::AccessibilityPlugin;

// Import framework components
use rusty_whip::gui_framework::{
    VulkanContext, Scene, RenderObject, EventBus, BusEvent, EventHandler,
    InteractionController,
};
use rusty_whip::Vertex;
use rusty_whip::gui_framework::context::vulkan_setup::{setup_vulkan, cleanup_vulkan};

use std::sync::{Arc, Mutex};
use std::any::Any;
use std::collections::HashMap;
use ash::vk; // Keep for vk::Extent2D

// --- Bevy Resources to hold existing framework components (Temporary Bridge) ---
#[derive(Resource, Clone)]
struct VulkanContextResource(Arc<Mutex<VulkanContext>>);

#[derive(Resource, Clone)]
struct SceneResource(Arc<Mutex<Scene>>);

#[derive(Resource, Clone)]
struct EventBusResource(Arc<EventBus>); // Old event bus

struct PlaceholderRenderer;
impl PlaceholderRenderer { fn new(_: &VulkanContext, _: vk::Extent2D, _: &Scene) -> Self { Self } }
impl EventHandler for PlaceholderRenderer { fn handle(&mut self, _: &BusEvent) {} fn as_any(&self) -> &dyn Any { self } }

#[derive(Resource, Clone)]
struct RendererResource(Arc<Mutex<PlaceholderRenderer>>); 

#[derive(Resource, Clone)]
struct InteractionControllerResource(Arc<Mutex<InteractionController>>); // Old controller

#[derive(Resource, Clone)]
struct ClickRouterResource(Arc<Mutex<ClickRouter>>); // Old click router

// --- Temporary Click Router (Kept from old main.rs) ---
// Will be replaced by Bevy events/systems later (Task 6.3)
struct ClickRouter {
    callbacks: HashMap<usize, Box<Mutex<dyn FnMut(usize, Option<usize>) + Send + 'static>>>,
}
impl ClickRouter {
    fn new() -> Self { ClickRouter { callbacks: HashMap::new() } }
    pub fn register_click_handler(
        &mut self, object_id: usize,
        callback: impl FnMut(usize, Option<usize>) + Send + 'static,
    ) {
        info!("[ClickRouter] Registering handler for object ID: {}", object_id);
        self.callbacks.insert(object_id, Box::new(Mutex::new(callback)));
    }
}
impl EventHandler for ClickRouter { // Listens to old EventBus
    fn handle(&mut self, event: &BusEvent) {
        if let BusEvent::ObjectClicked(clicked_id, instance_id) = event {
            info!("[ClickRouter] Received ObjectClicked event for ID: {}", clicked_id);
            if let Some(callback_mutex_box) = self.callbacks.get(clicked_id) {
                info!("[ClickRouter] Found handler for ID: {}. Locking and executing...", clicked_id);
                match callback_mutex_box.lock() {
                    Ok(mut callback_guard) => { (*callback_guard)(*clicked_id, *instance_id); }
                    Err(e) => { error!("[ClickRouter] Error: Could not lock callback mutex for ID {}: {}", clicked_id, e); }
                }
            } else { info!("[ClickRouter] No handler registered for ID: {}", clicked_id); }
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

// --- Temporary Scene Event Handler (Kept from old main.rs) ---
struct SceneEventHandler { scene_ref: Arc<Mutex<Scene>> }
impl EventHandler for SceneEventHandler { // Listens to old EventBus
    fn handle(&mut self, event: &BusEvent) {
        match event {
            BusEvent::ObjectMoved(index, delta, instance_id) => {
                if let Ok(mut scene) = self.scene_ref.lock() {
                    scene.translate_object(*index, delta[0], delta[1], *instance_id);
                } else { warn!("[SceneEventHandler] Could not lock scene mutex for ObjectMoved."); }
            }
            _ => {}
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

// --- Temporary Hotkey Action Handler (Kept from old main.rs) ---
struct HotkeyActionHandler { /* No proxy needed */ }
impl EventHandler for HotkeyActionHandler { // Listens to old EventBus
    fn handle(&mut self, event: &BusEvent) {
        if let BusEvent::HotkeyPressed(Some(action)) = event {
            info!("[HotkeyActionHandler] Hotkey Action Received: {}", action);
            if action == "CloseRequested" {
                info!("[HotkeyActionHandler] CloseRequested action received via old EventBus. App should exit via WindowCloseRequested.");
            }
        }
    }
    fn as_any(&self) -> &dyn Any { self }
}

fn main() {
    info!("Starting Rusty Whip with Bevy integration (Bevy 0.15)...");

    // --- Initialize old framework components ---
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));
    let event_bus = Arc::new(EventBus::new());
    let scene = {
        let bus_clone = event_bus.clone();
        Arc::new(Mutex::new(Scene::new(bus_clone)))
    };
    let controller = Arc::new(Mutex::new(InteractionController::new()));
    let click_router = Arc::new(Mutex::new(ClickRouter::new()));

    // --- Populate Scene (Example from old main) ---
    let mut triangle_id_opt: Option<usize> = None;
    let mut square_id_opt: Option<usize> = None;
    { // Scope for scene_guard
        let mut scene_guard = scene.lock().expect("Failed to lock scene for initial population");
        let width = 600.0; let height = 300.0;
        scene_guard.width = width; scene_guard.height = height;
        // Add objects (background, triangle, square, instances...)
        // (Code omitted for brevity - same as your original main.rs)
        scene_guard.add_object(RenderObject { vertices: vec![ Vertex { position: [0.0, 0.0] }, Vertex { position: [0.0, height] }, Vertex { position: [width, height] }, Vertex { position: [width, 0.0] }, ], vertex_shader_filename: "background.vert.spv".to_string(), fragment_shader_filename: "background.frag.spv".to_string(), depth: 0.0, on_window_resize_scale: true, on_window_resize_move: false, offset: [0.0, 0.0], is_draggable: false, instances: Vec::new(), visible: true, });
        let triangle_id = scene_guard.add_object(RenderObject { vertices: vec![ Vertex { position: [275.0, 125.0] }, Vertex { position: [300.0, 175.0] }, Vertex { position: [325.0, 125.0] }, ], vertex_shader_filename: "triangle.vert.spv".to_string(), fragment_shader_filename: "triangle.frag.spv".to_string(), depth: 1.0, on_window_resize_scale: false, on_window_resize_move: true, offset: [0.0, 0.0], is_draggable: true, instances: Vec::new(), visible: true, });
        triangle_id_opt = Some(triangle_id);
        let square_id = scene_guard.add_object(RenderObject { vertices: vec![ Vertex { position: [100.0, 50.0] }, Vertex { position: [100.0, 100.0] }, Vertex { position: [150.0, 100.0] }, Vertex { position: [150.0, 50.0] }, ], vertex_shader_filename: "square.vert.spv".to_string(), fragment_shader_filename: "square.frag.spv".to_string(), depth: 2.0, on_window_resize_scale: false, on_window_resize_move: true, offset: [0.0, 0.0], is_draggable: true, instances: Vec::new(), visible: true, });
        square_id_opt = Some(square_id);
        scene_guard.add_instance(triangle_id, [50.0, 50.0]);
        scene_guard.add_instance(square_id, [100.0, 0.0]);
    }

    // --- Register Click Handlers (Example from old main) ---
    { // Scope for router_guard
        let mut router_guard = click_router.lock().expect("Failed to lock ClickRouter for registration");
        // Register handlers for triangle_id_opt and square_id_opt...
        // (Code omitted for brevity - same as your original main.rs)
        if let Some(id) = triangle_id_opt { let mut count = 0; router_guard.register_click_handler(id, move |obj_id, inst_id| { count += 1; info!("---> Triangle (ID: {}) clicked! Instance: {:?}. Count: {}", obj_id, inst_id, count); }); }
        if let Some(id) = square_id_opt { router_guard.register_click_handler(id, |obj_id, inst_id| { info!("---> Square (ID: {}) clicked! Instance: {:?}", obj_id, inst_id); }); }
    }

    // --- Subscribe old handlers to the old EventBus ---
    let scene_event_handler = SceneEventHandler { scene_ref: scene.clone() };
    event_bus.subscribe_handler(scene_event_handler);
    let hotkey_handler = HotkeyActionHandler {};
    event_bus.subscribe_handler(hotkey_handler);
    event_bus.subscribe_arc(click_router.clone()); // Subscribe the click router

    // --- Build Bevy App ---
    App::new()
        // == Plugins ==
        .add_plugins((
            LogPlugin { level: Level::INFO, filter: "naga=warn,bevy_app=info,bevy_ecs=info,rusty_whip=debug".to_string(), ..default() },
            AccessibilityPlugin, // Often needed by windowing/input
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Rusty Whip (Bevy 0.15 Integration)".into(), // Set title directly
                    resolution: WindowResolution::new(600.0, 300.0),
                    present_mode: PresentMode::AutoVsync,
                    // mode: WindowMode::Windowed, // This is default
                    ..default() // Use bevy_utils::default
                }),
                ..default() // Use bevy_utils::default
            },
            WinitPlugin::<WakeUp>::default(), // Manages event loop and window interaction
            InputPlugin::default(),
        ))
        // == Resources ==
        .insert_resource(VulkanContextResource(vulkan_context))
        .insert_resource(SceneResource(scene))
        .insert_resource(EventBusResource(event_bus))
        .insert_resource(InteractionControllerResource(controller))
        .insert_resource(ClickRouterResource(click_router))
        // RendererResource inserted by create_renderer_system
        // == Startup Systems ==
        .add_systems(Startup,
            (
                // Use .pipe() to pass results/errors and ensure order
                setup_vulkan_system.pipe(create_renderer_system),
            ).chain() // Ensures sequential execution if more systems added here
        )
        // == Update Systems ==
        .add_systems(Update,
            (
                // Temporary bridge for old input controller (Currently NO-OP)
                // winit_event_bridge_system, // Keep commented out for now
                // System to handle window close requests -> AppExit
                handle_close_request,
                // System to handle resize events (needed to update Scene dimensions)
                handle_resize_system,
            )
        )
        // == Rendering System (runs late) ==
        .add_systems(Last, render_trigger_system)
        // == Shutdown System ==
        .add_systems(Last, cleanup_system.run_if(on_event::<AppExit>))
        // == Run the App ==
        .add_event::<KeyboardFocusLost>()
        .run();
}

// --- Bevy Systems ---

/// Startup system: Initializes Vulkan using the primary window handle.
fn setup_vulkan_system(
    vk_context_res: Res<VulkanContextResource>,
    primary_window_q: Query<Entity, With<PrimaryWindow>>,
    // Use NonSend<WinitWindows> to get the raw winit::Window reference
    winit_windows: NonSend<WinitWindows>,
) -> Result<(), String> { // Return Result for piping
    info!("Running setup_vulkan_system...");
    let primary_entity = primary_window_q.get_single()
        .map_err(|e| format!("Failed to get primary window entity: {}", e))?;

    // Get the underlying winit::Window reference
    let winit_window = winit_windows.get_window(primary_entity)
        .ok_or_else(|| "Failed to get winit window reference from WinitWindows".to_string())?;

    // Lock the mutex and call the setup function
    match vk_context_res.0.lock() {
        Ok(mut vk_ctx_guard) => {
            // Pass the winit::Window reference
            setup_vulkan(&mut vk_ctx_guard, winit_window);
            info!("Vulkan setup complete.");
            Ok(()) // Indicate success
        }
        Err(e) => {
            let err_msg = format!("Failed to lock VulkanContext mutex for setup: {}", e);
            error!("{}", err_msg);
            Err(err_msg) // Propagate error
        }
    }
}

/// Startup system (piped): Creates the Renderer instance.
fn create_renderer_system(
    In(setup_result): In<Result<(), String>>, // Get result from previous system
    mut commands: Commands,
    vk_context_res: Res<VulkanContextResource>,
    scene_res: Res<SceneResource>,
    event_bus_res: Res<EventBusResource>,
    primary_window_q: Query<&Window, With<PrimaryWindow>>,
) {
    if let Err(e) = setup_result {
        error!("Skipping renderer creation due to Vulkan setup error: {}", e);
        // Consider sending AppExit event here if Vulkan is critical
        // world.send_event(AppExit::Error); // Or similar
        return;
    }
    info!("Running create_renderer_system...");

    let primary_window = primary_window_q.get_single().expect("Primary window not found");
    let extent = vk::Extent2D { width: primary_window.physical_width(), height: primary_window.physical_height() };

    // Lock resources needed for Renderer::new
    // !! IMPORTANT !!: Renderer::new expects &mut VulkanContext.
    // Locking the Arc<Mutex<>> only gives us a MutexGuard, not &mut VulkanContext.
    // This requires refactoring Renderer::new or VulkanContext later.
    let vk_ctx_guard = vk_context_res.0.lock().expect("Failed to lock VulkanContext");
    let scene_guard = scene_res.0.lock().expect("Failed to lock Scene");

    // Create the placeholder renderer instance (using the globally defined struct)
    let renderer_instance = PlaceholderRenderer::new(&vk_ctx_guard, extent, &scene_guard);
    warn!("Using placeholder Renderer creation due to &mut VulkanContext requirement mismatch.");

    // Wrap in Arc<Mutex> and insert as resource
    let renderer_arc = Arc::new(Mutex::new(renderer_instance));
    commands.insert_resource(RendererResource(renderer_arc.clone()));

    // Subscribe the (placeholder) renderer to the old event bus
    event_bus_res.0.subscribe_arc(renderer_arc);

    info!("Renderer resource created and subscribed (using placeholder).");
}

/// Update system: Handles window resize events.
fn handle_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    // Get resources directly. Use Option<> in case they don't exist yet/failed setup.
    renderer_res_opt: Option<ResMut<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>, // Use Option
    scene_res_opt: Option<ResMut<SceneResource>>, // Use Option and ResMut
) {
    // Get resources *outside* the loop to avoid move errors
    let Some(renderer_res) = renderer_res_opt else { return; };
    let Some(vk_context_res) = vk_context_res_opt else { return; };
    let Some(scene_res) = scene_res_opt else { return; };
    
    for event in resize_reader.read() {
        info!("WindowResized event: {:?}", event);
        if event.width > 0.0 && event.height > 0.0 {
            // Lock resources needed for resize_renderer
            // Still faces the &mut VulkanContext issue.
            if let (Ok(_renderer_guard), Ok(_vk_ctx_guard), Ok(mut scene_guard)) = (
                renderer_res.0.lock(), // Lock the Mutex from ResMut
                vk_context_res.0.lock(),
                scene_res.0.lock() // Lock the Mutex from ResMut
            ) {
                warn!("Calling placeholder resize logic due to &mut VulkanContext requirement mismatch.");
                // --- HACK/Placeholder ---
                // Actual call would need refactoring:
                // renderer_guard.resize_renderer(&mut vk_ctx_guard, &mut scene_guard, event.width as u32, event.height as u32);
                // Placeholder: Update scene dimensions directly
                scene_guard.update_dimensions(event.width as u32, event.height as u32);
                // --- End HACK ---
            } else {
                warn!("Could not lock resources for resize handling.");
            }
        }
    }
}


/// Update system (Temporary): Bridges Winit events to the old InteractionController.
/// Currently NO-OP. Will be replaced by Bevy Input systems (Task 6.3).
fn winit_event_bridge_system(
    // Parameters omitted - system is inactive
) {
    // This system remains inactive. Interaction logic will be rebuilt using Bevy Input.
}

/// Update system: Triggers rendering via the old Renderer.
fn render_trigger_system(
    renderer_res_opt: Option<Res<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    scene_res_opt: Option<Res<SceneResource>>,
) {
    // Get resources using if let Some(...) else { return; } pattern
    if let (Some(renderer_res), Some(vk_context_res), Some(scene_res)) =
        (renderer_res_opt, vk_context_res_opt, scene_res_opt)
    {
        // Lock resources directly inside the if let condition to manage lifetimes
        if let (Ok(mut _renderer_guard), Ok(_vk_ctx_guard), Ok(_scene_guard)) = (
            renderer_res.0.lock(),
            vk_context_res.0.lock(),
            scene_res.0.lock(),
        ) {
            // --- HACK/Placeholder ---
            // Actual call needs refactor: _renderer_guard.render(&mut _vk_ctx_guard, &_scene_guard);
            // Placeholder log:
            // info!("render_trigger_system: Would call render (placeholder).");
            // --- End HACK ---
        } else {
            warn!("Could not lock resources for rendering trigger.");
        }
    }
}

/// Update system: Handles WindowCloseRequested events -> AppExit.
fn handle_close_request(
    mut ev_close: EventReader<WindowCloseRequested>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    if ev_close.read().next().is_some() {
        info!("WindowCloseRequested detected, sending AppExit.");
        ev_app_exit.send(AppExit::Success);
    }
}

/// System running on AppExit: Cleans up resources.
fn cleanup_system(
    // Use Option<Res<...>> for resources that might not exist if setup failed
    renderer_res_opt: Option<Res<RendererResource>>,
    vk_context_res_opt: Option<Res<VulkanContextResource>>,
    event_bus_res_opt: Option<Res<EventBusResource>>,
) {
    info!("Running cleanup_system...");

    // 1. Clear the old event bus
    if let Some(event_bus_res) = event_bus_res_opt {
        event_bus_res.0.clear();
        info!("Old EventBus cleared.");
    }

    // Get Vulkan context resource (needed for both renderer and final cleanup)
    let Some(vk_context_res) = vk_context_res_opt else {
        warn!("VulkanContext resource not found for cleanup.");
        return; // Cannot proceed without Vulkan context
    };

    // 2. Cleanup Renderer (must happen before Vulkan cleanup)
    if let Some(_renderer_res) = renderer_res_opt { // Use _renderer_res
        if let Ok(vk_ctx_guard) = vk_context_res.0.lock() {
            warn!("Renderer cleanup skipped due to Arc<Mutex>/&mut VulkanContext issues. Requires refactor.");
            // --- HACK/Placeholder ---
            // Actual logic: renderer_instance.cleanup(&mut vk_ctx_guard);
            // --- End HACK ---
        } else {
            error!("Could not lock VulkanContext for Renderer cleanup step.");
        }
    } else {
        info!("Renderer resource not found for cleanup.");
    }

    // 3. Cleanup Vulkan Context
    if let Ok(mut _vk_ctx_guard) = vk_context_res.0.lock() { // Keep mut and prefix with _
        info!("Calling cleanup_vulkan...");
        cleanup_vulkan(&mut _vk_ctx_guard); // Pass the mutable guard
        info!("cleanup_vulkan finished.");
    } else {
        error!("Could not lock VulkanContext for final cleanup.");
    }

    info!("Cleanup complete.");
}