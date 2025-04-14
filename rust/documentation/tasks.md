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

## Task 5.5: Migrate Scene and Event Management to Bevy ECS
- **Goal**: Replace the custom `Scene`, `ElementPool`, `RenderObject`, `EventBus`, `EventHandler`, `InteractionController`, `ClickRouter`, and associated logic with `bevy_ecs`. Refactor existing rendering and interaction logic into Bevy Systems operating on Components within a `bevy_ecs::World`.
- **Status**: **Not Started**
- **Steps**:
    1.  **Add Dependency:** Add `bevy_ecs` to `Cargo.toml`. Consider if `bevy_app` is useful for setup.
    2.  **Define Core Components:** Create initial component structs (e.g., `Transform { position, depth, scale }`, `RenderableShape { vertices, shader_info }`, `Visibility { visible }`, `Interaction { clickable, draggable }`, potentially `Hierarchy { parent }`).
    3.  **Define Core Resources:** Identify and define global data needed by systems (e.g., `WindowSize { width, height }`, `InputState`, potentially `VulkanHandles`).
    4.  **Refactor `main.rs`:**
        *   Replace `Scene`/`EventBus` setup with `bevy_ecs::World` initialization (or `bevy_app::App`).
        *   Replace initial object creation with entity spawning (`world.spawn((Transform{...}, RenderableShape{...}, ...))`).
        *   Remove old event subscriptions (`HotkeyActionHandler`, `SceneEventHandler`, `ClickRouter`).
        *   Integrate the Bevy schedule (e.g., `world.run_schedule(...)` or `app.update()`) into the `winit` event loop.
        *   Add core systems to the schedule.
    5.  **Create Core Systems:**
        *   **InputSystem:** Processes `winit` events, updates `InputState` resource, maybe sends Bevy input events (e.g., `KeyboardInput`, `MouseInput`).
        *   **InteractionSystem:** Queries entities with `Transform` and `Interaction` components. Uses `InputState` resource for mouse position/clicks. Performs hit-testing. Manages drag state (potentially via a `Dragging { target: Entity }` component or resource). Publishes Bevy events (e.g., `EntityClicked { entity: Entity }`, `EntityDragged { entity: Entity, delta: Vec2 }`).
        *   **MovementSystem:** Responds to `EntityDragged` events (or similar) by updating `Transform` components.
        *   **HotkeySystem:** Reads `InputState` resource, checks against loaded hotkey configuration (config loading logic might become a setup system or resource), publishes Bevy events like `HotkeyActionTriggered { action: String }`.
        *   **ApplicationControlSystem:** Responds to `HotkeyActionTriggered` (e.g., for "CloseRequested") or `winit` events to manage application lifecycle (e.g., sending `AppExit` event).
        *   **RenderingSystem:** Queries entities with `Transform`, `RenderableShape`, `Visibility`. Collects data for visible entities and passes it to the Vulkan backend (`Renderer`/`BufferManager`).
    6.  **Refactor Vulkan Backend (`Renderer`, `BufferManager`, etc.):**
        *   Modify interfaces to accept lists/iterators of component data (e.g., `Vec<(TransformData, ShapeData)>`) from the `RenderingSystem` instead of pulling from the old `Scene`.
        *   The core Vulkan API calls remain, but data sourcing changes. Cleanup logic needs careful review.
    7.  **Remove Obsolete Code:** Delete `src/gui_framework/scene/`, `src/gui_framework/event_bus.rs`, `src/gui_framework/interaction/controller.rs`. Remove old handlers and structs (`RenderObject`, `InstanceData`, `EventHandler`, etc.) from `main.rs` and `lib.rs`/`gui_framework/mod.rs`. Update `gui_framework/mod.rs` exports.
- **Impact:** This is a major refactor touching most framework parts. It replaces the core data storage and logic flow with an ECS pattern, enabling better composition and scalability. Requires updating all subsequent tasks.

## Task 6: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layout/shaping and implement a custom Vulkan bitmap glyph atlas renderer. Display static sample text (English/Chinese placeholder).
- **Affected Modules**: New `src/gui_framework/rendering/text_renderer/mod.rs`, `src/gui_framework/rendering/text_renderer/glyph_atlas.rs`, `src/gui_framework/rendering/buffer_manager.rs` (integration), `src/gui_framework/rendering/render_engine.rs` (integration), `src/gui_framework/scene/scene.rs` (add text objects), `src/main.rs` (testing, initialization), New shaders (`glyph.vert`, `glyph.frag`).
- **Status**: Not started
- **Steps**:
    1. Add dependencies: `cosmic-text`, `fontdb`, `swash`, potentially `rectangle-pack`.
    2. Create `glyph_atlas.rs` module (Vulkan `vk::Image` atlas, rasterization via `swash`, packing, GPU upload, UV tracking).
    3. Create `text_renderer.rs` module (`cosmic_text::FontSystem`, `SwashCache`, API for text buffers, layout triggering, glyph atlas interaction, vertex generation).
    4. Integrate text renderer into `main.rs` initialization and the rendering loop (`render_engine.rs` / `buffer_manager.rs`): resource management, dynamic vertex buffers, pipeline/shaders, atlas binding, drawing.
    5. Modify `RenderObject` or create `TextObject` in `scene.rs`.
    6. Test in `main.rs`: Create text objects, verify rendering.
- **Constraints**: Focus on bitmap rendering. Requires significant Vulkan integration. Defer SDF rendering.

## Task 7: Text Handling - Editing & Interaction
- **Goal**: Integrate `yrs` (`YText`) for collaborative data storage. Implement basic mouse/keyboard editing for text objects.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs` (manage `YText`), `src/gui_framework/rendering/text_renderer/mod.rs` (update from `YText`, render cursor), `src/main.rs` (testing).
- **Status**: Not started
- **Steps**:
    1. Add dependency: `yrs`.
    2. Replace/Augment text storage in `scene.rs` with `yrs::Text`.
    3. Modify `InteractionController` to track focus, handle keyboard input (generate `YText` ops), handle mouse clicks (calculate position, set cursor).
    4. Modify `TextRenderer` / main loop to observe `YText` changes and trigger re-layout/re-rendering.
    5. Modify `TextRenderer` to draw cursor.
    6. Test in `main.rs`: Create editable text object, edit, move cursor, verify updates.
- **Constraints**: Basic editing first. Defer complex selections, advanced cursor movement. Focus on local editing; P2P sync later.

## Task 8: Implement Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey (e.g., Ctrl+Space), with dynamic options.
- **Affected Modules**: New `src/gui_framework/ui/pie_menu.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs` (or UI manager), `src/gui_framework/rendering/text_renderer/mod.rs` (labels), New shaders (`pie_menu.vert`, `pie_menu.frag`), `src/gui_framework/event_bus.rs`, `src/gui_framework/mod.rs`, `src/main.rs` (handling).
- **Status**: Not started
- **Steps**:
    1. Create `pie_menu.rs` (`PieMenu`, `PieOption`, rendering logic using shapes/text).
    2. Update `Scene` or UI manager: Add `active_menu: Option<PieMenu>`.
    3. Update `InteractionController`: Handle `BusEvent::HotkeyPressed("show_pie_menu")`. Publish `BusEvent::ShowPieMenu(position, context)`.
    4. Update `Scene`/UI manager: Subscribe to `ShowPieMenu`, create `PieMenu`, set `active_menu`.
    5. Update `InteractionController`: Handle clicks when `active_menu` is Some. Determine selection, publish `BusEvent::MenuOptionSelected(action_id)`. Clear `active_menu`.
    6. Update `Renderer`/`BufferManager`/`TextRenderer`: Draw `active_menu`.
    7. Test: Trigger menu, verify display, select option, verify event.
- **Constraints**: Event-driven; uses Task 6 text rendering. Requires hotkey system (Task 4).

## Task 9: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the `gui_framework` for managing resizable layout regions. Provide an API for end-users to enable and configure it.
- **Affected Modules**: New `src/gui_framework/ui/dividers.rs`, `src/gui_framework/scene/scene.rs` (or UI manager), `src/gui_framework/interaction/controller.rs`, `src/gui_framework/mod.rs`, `src/main.rs` (testing/handling).
- **Status**: Not started
- **Steps**:
    1.  Define `Divider`, `DividerSystem` in `dividers.rs`.
    2.  Integrate `DividerSystem` into `Scene` or UI manager. Provide API (`enable_dividers`, `add_divider`).
    3.  Use `Scene::add_object` for visual representation (draggable rectangles).
    4.  Handle `ObjectMoved` for dividers. Constrain movement, calculate region changes, publish `RegionResized` event.
    5.  Expose configuration and `RegionResized` handling via public API.
    6.  Test in `main.rs`: Enable system, add divider, drag, log `RegionResized`.
- **Constraints**: Builds on framework primitives. Needs clear API separation. Layout logic calculates dimensions; rendering content within regions is application's responsibility.

## Task 10: Enhance Prompt Tool - Code Signature Stripping
- **Goal**: Add a new option to `utilities/llm_prompt_tool.sh` that processes Rust files, stripping out function bodies while retaining signatures, `impl` blocks, structs, comments, and other surrounding code.
- **Affected Modules**: `utilities/llm_prompt_tool.sh`.
- **Status**: Not started
- **Steps**:
    1.  Add new menu option (e.g., "5) Get Code Signatures").
    2.  Implement `.rs` file discovery.
    3.  Implement function body stripping logic (identify `fn`, replace `{...}` with placeholder, preserve surrounding code).
    4.  Integrate into file processing loop.
    5.  Comment limitations (macros, formatting).
    6.  Test on project codebase.
- **Constraints**: Bash implementation (`sed`, `awk`, etc.). Aim for common cases.

## Deferred Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom).
- Rendering: Depth-based re-sorting of renderables.
- Collaboration: Full P2P synchronization of `yrs` CRDT operations.
- UI: Context switching, undo/redo system.
- General: Performance optimizations (e.g., instance buffer resizing).