# Tasks for `rusty_whip` GUI Framework Enhancements

## Overview
These tasks enhance `gui_framework` to support a future divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building specific UIs atop it. Recent focus: Implementing event bus, refactoring rendering, logical grouping, batch updates via events, visibility toggling, and a configurable hotkey system. The main event loop now uses `EventLoop::run` within `main.rs`.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete**
- **Summary**: Implemented `EventBus`, converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`.

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`, `src/gui_framework/mod.rs`.
- **Status**: **Complete**
- **Summary**: Created `group.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`.

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Affected Modules**: `src/gui_framework/scene/group.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs` (event handling).
- **Status**: **Complete**
- **Summary**: Added `visible` state flag to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` variant to `BusEvent` (using `Arc<dyn Any + Send + Sync>`). Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` (subscribed in `main.rs`) to handle `FieldUpdated` and modify `RenderObject` state. Tested in `main.rs`.
- **Notes**: Implements batch *triggering*. Changing `depth` requires renderer re-sorting (not implemented). Changing `visible` requires renderer modification (Task 3.1).

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**: Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects.
- **Affected Modules**: `src/gui_framework/rendering/renderable.rs`, `src/gui_framework/rendering/buffer_manager.rs`, `src/gui_framework/rendering/command_buffers.rs`.
- **Status**: **Complete**
- **Summary**: Added `visible: bool` field to `Renderable`. Updated `BufferManager::new` to copy visibility state from `RenderObject` to `Renderable`. Modified `record_command_buffers` to check `renderable.visible` before issuing draw commands. Tested by setting initial visibility in `main.rs`.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file (`user/hotkeys.toml`) to map keys/modifiers to action strings, gracefully handling undefined hotkeys. Use `Escape` key for closing the window via event bus and proxy.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/interaction/hotkeys.rs`, `src/gui_framework/mod.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs`, `build.rs`, `Cargo.toml`.
- **Status**: **Complete**
- **Summary**: Added `toml` and `thiserror` dependencies. Created `hotkeys.rs` for config loading/parsing (`HotkeyConfig`, `HotkeyError`) and key formatting (`format_key_event`). Updated `InteractionController` to load config relative to executable (using path from `build.rs`), track modifier state (`current_modifiers`, handles `ModifiersChanged`), and publish `BusEvent::HotkeyPressed(Some(action_string))` on recognized key presses. Updated `build.rs` (`copy_user_files`) to copy `user/hotkeys.toml` to the target directory. Updated `main.rs` to use `EventLoop::run` and `EventLoopProxy<UserEvent>`, added `HotkeyActionHandler` subscriber that listens for `HotkeyPressed(Some("CloseRequested"))` and sends `UserEvent::Exit` via proxy to trigger clean shutdown. Tested `Escape`, `Ctrl+S`, `Alt+P`.
- **Constraints**: Uses `EventBus` and `EventLoopProxy`. Relies on `build.rs` copying config.

## Task 5: Add Generic Click Handling via Event Router
- **Goal**: Implement a generic mechanism to handle mouse clicks on any `RenderObject` by publishing an `ObjectClicked` event and providing an easy way for the application to register specific callback functions for different object IDs using a central router.
- **Affected Modules**: `src/gui_framework/event_bus.rs`, `src/gui_framework/interaction/controller.rs`, `src/main.rs` (for router definition, instantiation, and testing).
- **Status**: **Complete**
- **Steps**:
    1.  **Event Bus:** Add a new variant `ObjectClicked(usize, Option<usize>)` to the `BusEvent` enum in `src/gui_framework/event_bus.rs`. This event will carry the ID of the clicked object and the optional ID of the specific instance clicked.
    2.  **Interaction Controller:** Modify `InteractionController::handle_event` in `src/gui_framework/interaction/controller.rs`. In the `WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }` handler, when `scene.pick_object_at()` returns a target `(object_id, instance_id)`, publish `BusEvent::ObjectClicked(object_id, instance_id)` instead of `BusEvent::ObjectPicked`.
    3.  **Click Router (Definition):** Define a new struct `ClickRouter` within `src/main.rs`.
        *   Give it a field like `callbacks: std::collections::HashMap<usize, Box<dyn FnMut(usize, Option<usize>) + 'static>>`.
        *   Implement a `new()` function.
        *   Implement the `EventHandler` trait for `ClickRouter`. The `handle` method should check for `BusEvent::ObjectClicked`, look up the `object_id` in the `callbacks` map, and execute the associated closure if found.
    4.  **Click Router (API & Usage):**
        *   Implement a public method `register_click_handler(&mut self, object_id: usize, callback: impl FnMut(usize, Option<usize>) + 'static)` on `ClickRouter` in `src/main.rs` to add callbacks to the map.
        *   In `main.rs`, instantiate the `ClickRouter`, wrap it in `Arc<Mutex<>>`, and subscribe it to the `EventBus` using `event_bus.subscribe_arc()`.
        *   After creating scene objects and getting their IDs, lock the router and use `register_click_handler` to add at least one test callback (e.g., using `println!`) associated with a specific `RenderObject` ID.
    5.  **Testing:** Run the application. Click the `RenderObject` for which a callback was registered. Verify that the output from the test callback appears in the console.
- **Constraints**: Event-driven; relies on hit detection; uses a central router pattern defined in the application (`main.rs`) rather than the framework itself. Requires closures to have a `'static` lifetime.

## Task 6: Incremental Migration to Bevy Ecosystem

**Overall Goal:** Gradually replace custom framework components (windowing, input, scene management, event bus, math) with their equivalents from the Bevy ecosystem (`bevy_app`, `bevy_winit`, `bevy_input`, `bevy_ecs`, `bevy_math`, `bevy_log`, `bevy_reflect`, `bevy_utils`) while keeping the application functional at each stage. The custom Vulkan rendering backend remains but adapts its data inputs. **Crucially, this migration must strictly avoid integrating any part of Bevy's rendering stack (`bevy_render`, `bevy_pbr`, `bevy_sprite`, `bevy_ui`, `wgpu`, etc.). The application will rely *solely* on the custom Vulkan renderer.**

**Status:** Task 6.1 & 6.2 Complete. Task 6.3 Not Started.

### Task 6.1: Integrate Bevy App & Windowing (`bevy_app`, `bevy_winit`, `bevy_window`, Core Deps)

*   **Goal:** Replace the manual `winit` event loop and window creation with `bevy_app` and `bevy_winit`. The existing custom `Scene`, `EventBus`, `InteractionController`, `Renderer`, and `VulkanContext` will be kept *for now* and triggered from within the Bevy app structure using temporary bridging mechanisms.
*   **Status:** **Complete**
*   **Steps:**
    1.  **Add Dependencies:** Add `bevy_app`, `bevy_winit`, `bevy_window`. This will also pull in `bevy_ecs`, `bevy_utils`, `bevy_log`, `bevy_reflect`, `bevy_core`. Start with `MinimalPlugins` and add `WinitPlugin`. Check `bevy_window` docs for exact plugin usage.
        *   **Note:** It is essential *not* to use `DefaultPlugins`, as this would pull in Bevy's rendering plugins. Stick to the minimal required plugins like `WindowPlugin`, `WinitPlugin`, `InputPlugin`, `LogPlugin`, etc.
    2.  **Refactor `main.rs` (Entry Point):**
        *   Remove the manual `winit::EventLoop::run(...)` structure.
        *   Instantiate `bevy_app::App::new()`.
        *   Add `MinimalPlugins` and necessary windowing plugins (e.g., `WinitPlugin::default()`).
        *   Keep the existing `Arc<Mutex<...>>` containers for `VulkanContext`, `Scene`, `EventBus`, `InteractionController`, `Renderer`, `ClickRouter`. **Store these Arcs as Bevy Resources** (`app.insert_resource(scene_arc.clone())`, etc.).
        *   Add a Bevy `Startup` system (`setup_vulkan_system`) that:
            *   Takes `NonSend<WinitWindows>` (from `bevy_winit`), `ResMut<Arc<Mutex<VulkanContext>>>` as parameters.
            *   Retrieves the primary `winit::Window` handle (use `Query<&Window, With<PrimaryWindow>>` from `bevy_window`).
            *   Calls the existing `setup_vulkan` function, passing the `VulkanContext` and the `Window` handle.
        *   Add another `Startup` system (`create_renderer_system`) that runs *after* `setup_vulkan_system` (use `.after()`). It takes `Res<Arc<Mutex<VulkanContext>>>`, `Res<Arc<Mutex<Scene>>>`, `Res<Arc<Mutex<Renderer>>>` and creates the `Renderer` instance, storing it back in the resource.
        *   Add a Bevy `Update` system (`winit_event_bridge_system`) that:
            *   Takes `EventReader<bevy_window::RawWindowThreadEvent>` (confirm event name and if plugin needs config), or alternatively reads raw Winit events if necessary via custom runner integration.
            *   Takes `Res<Arc<Mutex<InteractionController>>>`, `Res<Arc<Mutex<Scene>>>`.
            *   For relevant Winit events, locks the `InteractionController` and calls its `handle_event` method, passing the (locked) `Scene` resource if needed. *This is the temporary input bridge.*
        *   Add a Bevy `Update` system (`render_trigger_system`) that runs late in the schedule (e.g., in `RenderStages::Render` or `Last`). It takes `Res<Arc<Mutex<Renderer>>>`, `Res<Arc<Mutex<Scene>>>`, `Res<Arc<Mutex<VulkanContext>>>` and calls `renderer.render(vk_ctx, scene)`.
        *   Add systems to handle `bevy_window::WindowCloseRequested` or `bevy_app::AppExit` events for clean shutdown. Vulkan cleanup (`cleanup_vulkan` and `renderer.cleanup`) needs to be triggered here, likely via an `OnExit` system for a specific state or listening to `AppExit`. Use `.run()` conditions based on `AppExit` reader.
        *   Call `app.run()`.
    3.  **Testability:**
        *   The application should compile and launch, showing the Winit window managed by Bevy.
        *   The initial scene should be rendered by the `render_trigger_system` calling the existing renderer.
        *   Input handling via the `winit_event_bridge_system` calling the old `InteractionController` *might* work, but could be fragile. Test clicking, dragging, hotkeys.
        *   Closing the window should trigger cleanup and exit.
*   **Outcome:** The app runs using `bevy_app`'s runner and `bevy_winit`, but the core logic still resides in the old structs accessed via Resources. Input is temporarily bridged.

### Task 6.2: Adopt Bevy Math (`bevy_math`)

*   **Goal:** Replace all usage of the `glam` crate with `bevy_math` types (`Vec2`, `Mat4`, etc.) throughout the *entire existing codebase* (including rendering logic, custom structs).
*   **Status:** **Complete**
*   **Steps:**
    1.  **Add Dependency:** Ensure `bevy_math` is available (pulled in by `bevy_app`). Remove direct `glam` dependency from `Cargo.toml`.
    2.  **Code Replacement:** Search and replace `glam::Vec2` -> `bevy_math::Vec2`, `glam::Mat4` -> `bevy_math::Mat4`, etc. Update `Vertex` struct. Update matrix methods (e.g., `orthographic_rh` might be slightly different).
    3.  **Compile and Fix:** Resolve any compiler errors resulting from the type changes.
*   **Testability:** The application should compile and run. Visually inspect that rendering positions, scaling, and dragging behaviour (if working) appear identical.
*   **Outcome:** The codebase now uses Bevy's standard math types, preparing for ECS component definitions.

### Task 6.3: Migrate Core Logic to Bevy ECS & Input

*   **Goal:** Remove the custom `Scene`, `ElementPool`, `RenderObject`, `EventBus`, `InteractionController`, `ClickRouter`. Replace them with Bevy ECS Components, Resources, Events, and Systems. Utilize `bevy_input` for cleaner input handling.
*   **Status:** Steps 1-6 Completed
*   **Steps:**
    1.  **Add `bevy_input`:** Add the `InputPlugin::default()` to the `App` in `main.rs`. Remove the temporary `winit_event_bridge_system`.
    2.  **Define Core Components:** Create component structs in `src/gui_framework/components/`:
        *   `Transform` (use `bevy_transform::components::Transform`).
        *   `ShapeData { vertices: Vec<Vertex>, shader_path: String }`.
        *   `Visibility`: **Define a custom `Visibility` component (e.g., `struct Visibility(pub bool);`). Do *not* use `bevy_render::view::Visibility` or any components from `bevy_render`.** Mark it with `#[derive(Component)]`.
        *   `Interaction { clickable: bool, draggable: bool }`. Mark them with `#[derive(Component)]`.
    3.  **Define Core Events:** Create event structs in `src/gui_framework/events/`: `EntityClicked { entity: Entity }`, `EntityDragged { entity: Entity, delta: Vec2 }`, `HotkeyActionTriggered { action: String }`. Add them via `app.add_event::<...>()`.
    4.  **Refactor Setup:** Remove the old `SceneResource`, `EventBusResource`, `InteractionControllerResource`, `ClickRouterResource` resources. Modify the `Startup` system(s) to spawn entities directly using `commands.spawn((Transform::from_xyz(...), ShapeData{...}, Interaction{...}, Visibility(...), ...))`. Load `HotkeyConfig` into a Bevy resource (`Res<HotkeyConfig>`).
    5.  **Create Input/Interaction Systems:**
        *   Create `interaction_system` (in `src/gui_framework/systems/`):
            *   Takes `Res<Input<MouseButton>>`, `EventReader<CursorMoved>` (from `bevy_window`), `Query<&Window, With<PrimaryWindow>>`.
            *   Queries entities with `&bevy_transform::components::Transform`, `&ShapeData`, `&Visibility` (custom), `&Interaction`.
            *   Performs hit-testing using Bevy input resources/events. Access window via query.
            *   Manages drag state (e.g., via a `Local<Option<Entity>>` or a `DraggingState` resource).
            *   Writes `EntityClicked` and `EntityDragged` events using `EventWriter`.
    6.  **Create Logic Systems:**
        *   Create `movement_system`: Reads `EventReader<EntityDragged>`, updates `Query<&mut bevy_transform::components::Transform>`.
        *   Create `hotkey_system`: Reads `Res<Input<KeyCode>>`, `Res<HotkeyConfig>`, writes `EventWriter<HotkeyActionTriggered>`. (Handle modifiers via `Input<KeyCode>.pressed()`).
        *   Create `app_control_system`: Reads `EventReader<HotkeyActionTriggered>` or `EventReader<WindowCloseRequested>`, sends `EventWriter<AppExit>`. Modify the exit handler in `main.rs` to listen for `AppExit`.
    7.  **Refactor Rendering Path:**
        *   Modify the `render_trigger_system` (rename to `rendering_system`):
            *   Queries entities with `&bevy_transform::components::Transform`, `&ShapeData`, `&Visibility` (our custom component).
            *   Collects data into a temporary structure (e.g., `Vec<RenderCommandData>`), sorts by depth (`Transform.translation.z`).
            *   Takes `Res<Arc<Mutex<Renderer>>>` (our custom Vulkan renderer wrapper), `Res<Arc<Mutex<VulkanContext>>>`.
            *   **Calls the *modified custom Vulkan* `Renderer::render` method**, passing the collected ECS component data (`render(vk_ctx, collected_render_data)`). **This system must *only* interact with the custom Vulkan renderer and must not involve any Bevy rendering systems, pipelines, or render graphs.**
        *   Adapt *custom* `Renderer`, `BufferManager` APIs: `render` now accepts `&[RenderCommandData]`. `BufferManager` needs significant rework: maybe manage buffers per `ShapeData` hash or via Entity ID mapping. Updates need to be driven by `Changed<Transform>` or similar for efficiency (requires `bevy_ecs` change detection features). Instancing needs ECS approach (e.g., component linking).
    8.  **Remove Obsolete Code:** Delete old `Scene`, `EventBus`, `InteractionController`, etc., modules/files and associated structs/resources.
*   **Testability:** Test incrementally: entity spawning, input state, hit-testing events, drag events, transform updates, hotkey events, rendering output reflecting ECS state.
*   **Outcome:** Core logic runs within ECS. Custom scene/event management removed. Input uses `bevy_input`. Rendering consumes ECS data via the custom Vulkan path only.

### Task 6.4: Integrate Logging & Reflection

*   **Goal:** Integrate `bevy_log` for consistent logging and `bevy_reflect` for potential future use (editors, serialization).
*   **Status:** Not Started
*   **Steps:**
    1.  **Logging:** Ensure `LogPlugin` is added (likely part of `MinimalPlugins` or `DefaultPlugins`). Replace `println!`/`eprintln!` calls with `info!`, `warn!`, `error!`, `debug!`, `trace!` macros from `bevy_log` (or `tracing`).
    2.  **Reflection:** Add `bevy_reflect::ReflectPlugin` to the app if not already present. Add `#[derive(Component, Reflect)]`, `#[derive(Resource, Reflect)]`, `#[derive(Event, Reflect)]` etc., to relevant structs/enums. Add `#[reflect(Component)]` attributes. Register types using `.register_type::<MyComponent>()` in a setup system or plugin.
*   **Testability:** Check console output for logs formatted by `bevy_log`. Add a simple system that accesses the `AppTypeRegistry` resource to verify types have been registered via reflection.
*   **Outcome:** Logging is standardized, and reflection capabilities are enabled for future development.

## Task 7: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layout/shaping and implement a custom Vulkan bitmap glyph atlas renderer integrated with the existing Vulkan backend. Display static sample text represented as Bevy ECS entities.
- **Status**: Not started
- **Affected Components/Systems/Resources**:
    - New Component: `Text { content: String, font_id: FontId, size: f32, color: Color, bounds: Option<Vec2>, .. }` (`#[derive(Component, Reflect)]`)
    - New Resource: `GlyphAtlasResource { texture: vk::Image, view: vk::ImageView, sampler: vk::Sampler, layout: ..., allocator: vk_mem::Allocator, ... }` (or managed within TextSystem)
    - New Resource: `FontServer` (using `cosmic_text::fontdb`, likely initialized at startup)
    - New Systems: `text_layout_system`, `text_rendering_system`.
    - Modified Systems: Main `rendering_system` needs integration point.
- **Steps**:
    1.  **Add Dependencies:** Add `cosmic-text`, `fontdb`, `swash`, `rectangle-pack` to `Cargo.toml`.
    2.  **Define `Text` Component:** Create the `Text` component struct to hold text data.
    3.  **Implement `GlyphAtlas` Logic:** Create a module/struct (`glyph_atlas.rs`) responsible for managing a Vulkan texture atlas. Implement functions to:
        *   Initialize the Vulkan `Image`, `ImageView`, `Sampler`.
        *   Use `rectangle-pack` to find space for new glyphs.
        *   Use `swash` and `FontServer` resource to rasterize glyphs.
        *   Upload glyph bitmaps to the Vulkan texture (using staging buffers).
        *   Store glyph UV coordinates.
        *   Manage this state potentially within a `GlyphAtlasResource`.
    4.  **Implement `FontServer` Resource:** Create a resource to load and manage fonts using `cosmic_text::FontSystem` and `fontdb`. Load default fonts at startup.
    5.  **Create `text_layout_system`:**
        *   Queries for entities with `(Changed<Text>, &bevy_transform::components::Transform)` components.
        *   Uses `FontServer` and `cosmic-text::Buffer::shape` for layout.
        *   Requests glyph rasterization/UVs from the `GlyphAtlasResource`.
        *   Stores layout results (e.g., positioned glyphs/quads) associated with the entity, perhaps in a temporary cache or another component (`TextLayoutOutput`).
    6.  **Create `text_rendering_system`:**
        *   Runs after `text_layout_system`.
        *   Queries entities with layout results (`TextLayoutOutput`) and `Visibility` (custom).
        *   Generates Vulkan vertex data for the glyph quads based on layout results and `Transform`.
        *   Updates dynamic Vulkan vertex buffers managed by this system (or dedicated resource).
        *   Integrates with the main `rendering_system`: Provides necessary data (vertex buffers, atlas descriptor set, pipeline) for the `rendering_system` to issue draw calls during the appropriate render phase. Requires defining a text-specific Vulkan pipeline and descriptor set layout for the glyph atlas sampler.
    7.  **Integrate into App:** Add the `Text` component, `FontServer`, `GlyphAtlasResource`, and the new systems to the Bevy `App`. Add necessary Vulkan setup for text pipeline/descriptors in a setup system.
    8.  **Test:** Spawn entities with `Transform` and `Text` components. Verify static text renders correctly. Modify `Text` component content and verify the display updates.
- **Constraints**: Requires Task 6 completion. Initial focus on non-wrapping, static text. Rendering must use the custom Vulkan backend.

## Task 8: Text Handling - Editing & Interaction
- **Goal**: Integrate `yrs` (`YText`) for collaborative data storage. Implement basic mouse/keyboard editing for text entities using Bevy Input and Systems.
- **Status**: Not started
- **Affected Components/Systems/Resources**:
    - Modified Component: `Text` (integration with Yrs data).
    - New Component: `EditableText` (marker), `Focus` (marker).
    - New Resource: `YrsDocResource { doc: yrs::Doc, text_map: HashMap<Entity, yrs::Text> }` (example structure).
    - New Systems: `text_editing_system`, `yrs_observer_system`.
    - Modified Systems: `interaction_system`, `text_layout_system`, `text_rendering_system`.
    - New Events: `TextFocusChanged { entity: Option<Entity> }`.
- **Steps**:
    1.  **Add Dependency:** Add `yrs` to `Cargo.toml`.
    2.  **Integrate `YrsDocResource`:** Set up a central Yrs document and a way to map Bevy `Entity` IDs to shared `yrs::Text` types within the document resource.
    3.  **Modify `Text` Component:** Adapt `Text` component to potentially reference its corresponding `yrs::Text` or have its `content` be driven by it.
    4.  **Modify `interaction_system`:** Query entities with `EditableText`. On click, determine the focused entity, add the `Focus` marker component to it (removing from others), and potentially send `TextFocusChanged` event. Calculate click position within text for cursor placement.
    5.  **Create `text_editing_system`:**
        *   Queries for the entity with the `Focus` component.
        *   Reads `Res<Input<KeyCode>>`, `EventReader<ReceivedCharacter>` (from `bevy_input`).
        *   Calculates cursor position based on clicks/input.
        *   Generates `yrs::Text` transaction operations (inserts, deletes) on the `YrsDocResource` based on keyboard input for the focused entity.
        *   Store cursor position state (perhaps in a component on the focused entity or the `Focus` component itself).
    6.  **Create `yrs_observer_system`:**
        *   Observes changes to the `yrs::Text` types within the `YrsDocResource` (using Yrs observers/subscriptions).
        *   When a `yrs::Text` changes, find the corresponding Bevy `Entity`.
        *   Update the `String` content within the entity's `Text` component to match the Yrs state. This `Changed<Text>` will trigger the `text_layout_system`.
    7.  **Cursor Rendering:** Modify `text_rendering_system` to query for the entity with `Focus` and cursor position data. Render a visual cursor (e.g., a simple quad) using the custom Vulkan renderer.
    8.  **Test:** Spawn an entity with `Text` and `EditableText`. Click to focus. Type characters, press backspace/delete. Verify the text updates visually and the cursor moves appropriately. Check underlying Yrs data if possible.
- **Constraints**: Requires Task 7. Focus on basic local editing (single cursor, basic input). P2P synchronization of the `YrsDocResource` is deferred.

## Task 9: Implement Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey, using Bevy ECS entities for UI elements and Bevy events/resources for state management.
- **Status**: Not started
- **Affected Components/Systems/Resources**:
    - New Components: `PieMenuUIElement` (marker), `PieOptionData { action: String, label: String, .. }`.
    - New Resource: `ActivePieMenuState { is_active: bool, position: Vec2, options: Vec<PieOptionData>, target_entity: Option<Entity> }` (or similar).
    - New Systems: `pie_menu_management_system`.
    - Modified Systems: `hotkey_system`.
    - New Events: `ShowPieMenu { position: Vec2, context_entity: Option<Entity> }`, `PieOptionSelected { action: String }`.
- **Steps**:
    1.  **Define Components/Resources/Events:** Create structs for menu state, options, and communication events. Add events via `app.add_event`. Initialize `ActivePieMenuState` resource.
    2.  **Modify `hotkey_system`:** On recognizing the menu hotkey (e.g., "Ctrl+Space"), determine context (e.g., hovered entity via `interaction_system` state?) and send a `ShowPieMenu` event with position and context.
    3.  **Create `pie_menu_management_system`:**
        *   **Activation:** Reads `ShowPieMenu` events. If received, populate the `ActivePieMenuState` resource with options based on context. Set `is_active = true`. Spawn necessary UI entities (background slices, text labels using `Text` component) with `Transform`, `ShapeData`/`Text`, `Visibility` (custom), and `PieMenuUIElement` components. Position them relative to `ActivePieMenuState.position`.
        *   **Interaction:** Reads `Res<Input<MouseButton>>` and cursor position (via `Query<&Window>`). If `ActivePieMenuState.is_active`, perform hit-testing against entities with `PieMenuUIElement` and `PieOptionData`. On click:
            *   Identify the selected `PieOptionData`.
            *   Send `PieOptionSelected` event with the action string.
            *   Set `ActivePieMenuState.is_active = false`.
        *   **Deactivation/Cleanup:** If `ActivePieMenuState.is_active` becomes false (or on any click outside?), despawn all entities with `PieMenuUIElement` component using `Commands`.
    4.  **Rendering:** Existing `rendering_system` and `text_rendering_system` (using custom Vulkan backend) will render the spawned UI entities based on their standard components (`Transform`, `ShapeData`, `Text`, `Visibility`).
    5.  **Action Handling:** Create other systems that listen for `PieOptionSelected` events and perform the corresponding actions.
    6.  **Test:** Trigger menu via hotkey. Verify UI elements appear correctly positioned. Click an option, verify `PieOptionSelected` event is sent and the menu disappears. Click outside the menu, verify it disappears.
- **Constraints**: Requires Task 6 and 7. UI element appearance defined by standard components. Rendering must use the custom Vulkan backend.

## Task 10: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the framework for managing resizable layout regions using Bevy ECS entities, components, and systems.
- **Status**: Not started
- **Affected Components/Systems/Resources**:
    - New Components: `Divider { axis: Axis, min: f32, max: f32, region_a: Entity, region_b: Entity }`, `LayoutRegion { bounds: Rect }`, `DraggableDivider` (marker, works with `Interaction` component).
    - New Systems: `divider_drag_system`, `layout_update_system`.
    - Modified Systems: `interaction_system` (or rely on `EntityDragged` for `DraggableDivider` entities).
    - New Events: `RegionResized { region_entity: Entity, new_bounds: Rect }`.
    - Resource: `DividerSystemConfig { enabled: bool }`.
- **Steps**:
    1.  **Define Components/Resources/Events:** Create structs for dividers, regions, configuration, and events. Add event via `app.add_event`. Initialize config resource.
    2.  **Setup System:** Create a system (or use `Startup`) that, if `DividerSystemConfig.enabled`, spawns divider entities (`Divider`, `Transform`, `ShapeData`, `Interaction { draggable: true }`, `DraggableDivider`, `Visibility` (custom)) and associated region entities (`LayoutRegion`, `Transform`). Link regions in `Divider` component. Use Bevy's `Parent`/`Children` components for hierarchy if needed.
    3.  **Create `divider_drag_system`:**
        *   Reads `EventReader<EntityDragged>` specifically for entities that *also* have the `DraggableDivider` component (use `Query` filtering).
        *   Reads `Query<(&Divider, &mut bevy_transform::components::Transform)>`.
        *   Applies the drag `delta` to the divider's `Transform`, constraining movement based on `Divider.axis` and `Divider.min`/`max`.
    4.  **Create `layout_update_system`:**
        *   Runs after the drag system (use `.after()`).
        *   Queries for dividers with `Changed<Transform>` and their associated `Divider` component.
        *   Queries `Query<&mut LayoutRegion>`.
        *   For each moved divider, calculate the new `bounds` for the linked `region_a` and `region_b` entities based on the divider's new transform.
        *   Update the `LayoutRegion` components on the region entities.
        *   Send `RegionResized` events for affected regions using `EventWriter`.
    5.  **Application Integration:** Application systems can query `LayoutRegion` components to position their own content or listen for `RegionResized` events to react to layout changes.
    6.  **Test:** Enable the system via config. Verify dividers and region entities are spawned. Drag a divider handle visually. Check that associated `LayoutRegion` bounds update correctly (log them). Verify `RegionResized` events are sent with correct data.
- **Constraints**: Requires Task 6. Handles layout bounds; rendering content within regions is the application's responsibility (using the custom Vulkan renderer).

## Task 11: Enhance Prompt Tool - Code Signature Stripping
- *(No changes needed, this task is external to the Rust codebase architecture)*
- **Goal**: Add a new option to `utilities/llm_prompt_tool.sh` that processes Rust files, stripping out function bodies while retaining signatures, `impl` blocks, structs, comments, and other surrounding code.
- **Affected Modules**: `utilities/llm_prompt_tool.sh`.
- **Status**: Not started
- **Steps**: *(remain the same)*
- **Constraints**: *(remain the same)*

## Deferred Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom), ensuring integration uses the *custom Vulkan backend* and not Bevy's rendering pipeline.
- Rendering: Optimizing *custom Vulkan* buffer updates using Bevy change detection (`Changed<T>`).
- Rendering: Advanced instancing techniques within ECS, implemented *within the custom Vulkan renderer* (avoiding Bevy's specific `BatchedMeshInstances` if it implies `bevy_render`).
- Collaboration: Full P2P synchronization of Components/Resources using `yrs` or other CRDTs, integrating with `YrsDocResource`.
- UI: Context switching (using Bevy `States<T>`), undo/redo system (potentially via command pattern integrated with ECS).
- General: Performance optimizations leveraging ECS parallelism and queries.