# Tasks for `rusty_whip` GUI Framework Enhancements

## Overview
These tasks enhance `gui_framework` to support a future divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building specific UIs atop it. Recent focus: Migrating core logic (input, state) to the Bevy ecosystem while retaining and adapting the custom Vulkan rendering backend.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Implemented `EventBus`, converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`. (Module deleted post-Bevy migration).

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`, `src/gui_framework/mod.rs`.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Created `group: Shader input fixes are **Done** (per commit).
        *   Follow-up: Resource Removal is **Not Done**.
        *   Follow-up: Vertex Updates is **Not Done**.
        *   Follow-up: Instancing is **Not Done**.
    *   **Conclusion:** The core migration and significant follow-up debugging/optimizations (caching, sync, resize) *are* complete according to the commit. The remaining items are further optimizations/features. The task status should be updated to **Complete**.
6.  **Task 6.4 (Logging & Reflection):**
    *   **Current Status:** "Not Started".
    *   **Code Check:** `main.rs` adds `LogPlugin` and uses `info!`, `warn!`, `error!` macros extensively. Reflection (`#[derive(Reflect)]`, registration) is not implemented in the provided snippets.
    *   **Conclusion:** The logging part is **Complete**. Reflection is **Not Started**. Update status accordingly.
7.  **Tasks 7-11:** Correctly marked "Not started".

**Updated `tasks.md`:**

```markdown
# Tasks for `rusty_whip` GUI Framework Enhancements

## Overview
These tasks enhance `gui_framework` to support a future divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building specific UIs atop it. Recent focus: Migrating core logic (input, state) to the Bevy ecosystem while retaining and adapting the custom Vulkan rendering backend.

*(.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`. (Module deleted post-Bevy migration).

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Affected Modules**: `src/gui_framework/scene/groupNote: Progress reflects state after commit implementing Vulkan resource caching and synchronization fixes.)*

## Task 1: Implement Event.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs` (event handling).
- **Status**: **Complete (Legacy - Pre-Bevy Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Implemented `EventBus`,)**
- **Summary**: Added `visible` state flag to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`. ( variant to `BusEvent` (using `Arc<dyn Any + Send + Sync>`). Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` (subscribed in `main.rs`) to handle `Code removed post-Bevy migration).

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **AffectedFieldUpdated` and modify `RenderObject` state. Tested in `main.rs`. (Modules deleted post-Bevy migration Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`,).
- **Notes**: Implemented batch *triggering*. Changing `depth` required renderer re-sorting. Changing `visible` required `src/gui_framework/mod.rs`.
- **Status**: **Complete (Legacy - Pre-Bev renderer modification (Task 3.1).

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**:y)**
- **Summary**: Created `group.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`. (Code removed post Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects.-Bevy migration).

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add
- **Affected Modules**: `src/gui_framework/rendering/renderable.rs`, `src/gui_framework/rendering/buffer_manager.rs`, `src/gui_framework/rendering/command_buffers.rs`.
- functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Affected **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Added `visible: bool` field Modules**: `src/gui_framework/scene/group.rs`, `src/gui_framework/scene/scene.rs`, to `Renderable`. Updated `BufferManager::new` to copy visibility state from `RenderObject` to `Renderable`. `src/gui_framework/event_bus.rs`, `src/main.rs` (event handling). Modified `record_command_buffers` to check `renderable.visible` before issuing draw commands. Tested by setting
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Added `visible` state flag initial visibility in `main.rs`. (Visibility now handled via Bevy component in Task 6.3).

 to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` variant to `BusEvent` (using `Arc<dyn Any + Send## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file (`user/hot + Sync>`). Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` (subscribedkeys.toml`) to map keys/modifiers to action strings, gracefully handling undefined hotkeys. Use `Escape` key for closing the window via event in `main.rs`) to handle `FieldUpdated` and modify `RenderObject` state. Tested in `main.rs`. (Code removed bus and proxy.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, ` post-Bevy migration).

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**:src/gui_framework/interaction/hotkeys.rs`, `src/gui_framework/mod.rs`, Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects. `src/gui_framework/event_bus.rs`, `src/main.rs`, `build.rs`, `Cargo.toml`.
- **Affected Modules**: `src/gui_framework/rendering/renderable.rs`, `src/gui_framework/rendering/buffer_manager.rs`, `src/gui_framework/rendering/command_buffers.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Added `toml` and `thiserror` dependencies. Created `hotkeys.rs` for config loading/parsing (`HotkeyConfig`, `HotkeyError`) and key formatting (`formatrs`.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Added `_key_event`). Updated `InteractionController` to load config relative to executable (using path from `build.rs`), trackvisible: bool` field to `Renderable`. Updated `BufferManager::new` to copy visibility state from `Render modifier state (`current_modifiers`, handles `ModifiersChanged`), and publish `BusEvent::HotkeyPressed(Some(action_string))`Object` to `Renderable`. Modified `record_command_buffers` to check `renderable.visible` before issuing on recognized key presses. Updated `build.rs` (`copy_user_files`) to copy `user/hotkeys.toml` to draw commands. Tested by setting initial visibility in `main.rs`. (Concept now handled via Bevy `Visibility` component). the target directory. Updated `main.rs` to use `EventLoop::run` and `EventLoopProxy<

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOUserEvent>`, added `HotkeyActionHandler` subscriber that listens for `HotkeyPressed(Some("CloseRequested"))` and sends `UserML file (`user/hotkeys.toml`) to map keys/modifiers to action strings, gracefully handling undefined hotkeys. Use `EscapeEvent::Exit` via proxy to trigger clean shutdown. Tested `Escape`, `Ctrl+S`, `Alt+P`. (Function` key for closing the window via event bus and proxy.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/interaction/hotkeys.rs`, `src/ality reimplemented via Bevy systems/resources in Task 6.3).
- **Constraints**: Used `EventBus` and `EventLoopgui_framework/mod.rs`, `src/gui_framework/event_bus.rs`, `src/Proxy`. Relied on `build.rs` copying config.

## Task 5: Add Generic Click Handling via Event Router
main.rs`, `build.rs`, `Cargo.toml`.
- **Status**: **Complete (Legacy - Pre-Bevy)**
- **Summary**: Added `toml` and `thiserror` dependencies. Created `hotkeys.rs` for config loading/- **Goal**: Implement a generic mechanism to handle mouse clicks on any `RenderObject` by publishing an `ObjectClicked` event and providing an easy wayparsing (`HotkeyConfig`, `HotkeyError`) and key formatting (`format_key_event`). Updated `InteractionController` to load config relative for the application to register specific callback functions for different object IDs using a central router.
- **Affected Modules**: `src/gui_framework/event_bus.rs`, `src/gui_framework/interaction/controller.rs`, `src/main to executable (using path from `build.rs`), track modifier state (`current_modifiers`, handles `ModifiersChanged`), and publish `.rs` (for router definition, instantiation, and testing).
- **Status**: **Complete (Legacy - Pre-Bevy)**
-BusEvent::HotkeyPressed(Some(action_string))` on recognized key presses. Updated `build.rs` (`copy **Summary**: Added `ObjectClicked` to `BusEvent`. Modified `InteractionController` to publish `ObjectClicked`. Defined_user_files`) to copy `user/hotkeys.toml` to the target directory. Updated `main.rs` to use `EventLoop::run` and `EventLoopProxy<UserEvent>`, added `HotkeyActionHandler` subscriber that `ClickRouter` in `main.rs` implementing `EventHandler` and registered callbacks. Tested successfully. (Functionality reimplemented via Be listens for `HotkeyPressed(Some("CloseRequested"))` and sends `UserEvent::Exit` via proxy to trigger clean shutdown. Testedvy systems/events in Task 6.3).
- **Constraints**: Event-driven; relied on hit detection; used a `Escape`, `Ctrl+S`, `Alt+P`. (Functionality now handled via Bevy `hotkey_system` central router pattern defined in the application (`main.rs`). Required closures to have a `'static` lifetime.

## and `HotkeyResource`).

## Task 5: Add Generic Click Handling via Event Router
- **Goal**: Implement a generic Task 6: Incremental Migration to Bevy Ecosystem

**Overall Goal:** Gradually replace custom framework components (windowing, input, mechanism to handle mouse clicks on any `RenderObject` by publishing an `ObjectClicked` event and providing an easy way scene management, event bus, math) with their equivalents from the Bevy ecosystem (`bevy_app`, `bevy_w for the application to register specific callback functions for different object IDs using a central router.
- **Affected Modules**: `src/gui_frameworkinit`, `bevy_input`, `bevy_ecs`, `bevy_math`, `bevy_/event_bus.rs`, `src/gui_framework/interaction/controller.rs`, `src/mainlog`, `bevy_reflect`, `bevy_utils`) while keeping the application functional at each stage. The custom.rs` (for router definition, instantiation, and testing).
- **Status**: **Complete (Legacy - Pre-Bevy)**
- Vulkan rendering backend remains but adapts its data inputs. **Crucially, this migration must strictly avoid integrating any part of Bevy's rendering **Summary**: Added `ObjectClicked` to `BusEvent`. Modified `InteractionController` to publish `ObjectClicked`. Defined stack (`bevy_render`, `bevy_pbr`, `bevy_sprite`, `bevy_ui`, `wgpu`, etc.). The application will rely *solely* on the custom Vulkan renderer.**

**Status:** Task 6. `ClickRouter` in `main.rs` implementing `EventHandler` to dispatch calls based on ID. Subscribed router1, 6.2 Complete. Task 6.3 **Partially Complete**. Task 6.4 **Partially Complete**.

### Task to `EventBus`. Registered test callbacks. (Functionality now handled via Bevy `interaction_system` sending `EntityClicked` events).

## 6.1: Integrate Bevy App & Windowing (`bevy_app`, `bevy_winit`, `bevy_window`, Core Deps)

*   **Goal:** Replace the manual `winit` event loop and Task 6: Incremental Migration to Bevy Ecosystem

**Overall Goal:** Gradually replace custom framework components (windowing, input, scene management, window creation with `bevy_app` and `bevy_winit`. Adapt the existing custom `Renderer` event bus, math) with their equivalents from the Bevy ecosystem (`bevy_app`, `bevy_w and `VulkanContext` to be managed as Bevy resources.
*   **Status:** **Complete**
*   **Summaryinit`, `bevy_input`, `bevy_ecs`, `bevy_math`, `bevy_:** Refactored `main.rs` to use `bevy_app::App`. Added necessary Bevy plugins (`Windowlog`, `bevy_reflect`, `bevy_utils`) while keeping the application functional at each stage. The custom Vulkan rendering backend remains but adapts its data inputs. **Crucially, this migration must strictly avoid integrating any part of Bevy's renderingPlugin`, `WinitPlugin`, `LogPlugin`, etc., *excluding rendering*). Stored `Arc<Mutex<VulkanContext>> stack (`bevy_render`, `bevy_pbr`, `bevy_sprite`, `bevy_ui`, `wgpu`, etc.). The application will rely *solely* on the custom Vulkan renderer.**

**Status:** **Complete**` and `Arc<Mutex<Renderer>>` as Bevy resources. Implemented `Startup` systems (`setup_vulkan_system`, (All sub-tasks implemented, including core rendering path adaptations and initial optimizations/fixes).

### Task 6.1: Integrate `create_renderer_system`) to initialize Vulkan and the custom renderer using Bevy resources and window handles. Implemented `Update Bevy App & Windowing (`bevy_app`, `bevy_winit`, `bevy_window`, Core Deps)

*   **Goal:** Replace the manual `winit` event loop and window creation with `bevy_app` and `bevy` system (`rendering_system`) to trigger the custom renderer. Implemented `Last` system (`cleanup_system`) triggered_winit`.
*   **Status:** **Complete**
*   **Summary:** Application runs using `bevy_app::App` by `AppExit` for Vulkan/Renderer cleanup.
*   **Outcome:** The app runs using `bevy_app`'s runner and `bevy_winit`. Vulkan context and the custom renderer are managed via Bevy resources and triggered and `bevy_winit`. Core non-rendering Bevy plugins added. Vulkan context initialized via `Startup by Bevy systems.

### Task 6.2: Adopt Bevy Math (`bevy_math`)

*` system. Renderer created via `Startup` system. Rendering triggered via `Last` system. Cleanup handled via `AppExit` event   **Goal:** Replace all usage of the `glam` crate with `bevy_math` types (`Vec2`, `Mat4`, etc.) throughout the *entire existing codebase* (including rendering logic, custom structs).
*   **Status:** **Complete system. Legacy framework structs initially bridged via Bevy resources.

### Task 6.2: Adopt Bevy Math**
*   **Summary:** Removed `glam` dependency. Replaced `glam` types with `bevy_math` (`bevy_math`)

*   **Goal:** Replace all usage of the `glam` crate with `bevy_math` types types (`Vec2`, `Mat4`) in all relevant modules (`Vertex`, `VulkanContext`, `Renderer`, `BufferManager`, `main.
*   **Status:** **Complete**
*   **Summary:** Codebase uses `bevy_math::{Vec2, Mat4}` etc. `glam` dependency removed.

### Task 6.3: Migrate.rs` systems, etc.). Updated matrix method calls where necessary.
*   **Outcome:** The codebase now uses Bevy's standard math types Core Logic to Bevy ECS & Input

*   **Goal:** Remove the custom `Scene`, `ElementPool`, `Render.

### Task 6.3: Migrate Core Logic to Bevy ECS & Input

*   **Goal:** RemoveObject`, `EventBus`, `InteractionController`, `ClickRouter`. Replace them with Bevy ECS Components, Resources, Events, and Systems. the custom `Scene`, `ElementPool`, `RenderObject`, `EventBus`, `InteractionController`, `ClickRouter`. Replace them with Bevy ECS Components, Resources, Events, and Systems. Utilize `bevy_input` for cleaner Utilize `bevy_input` for cleaner input handling. Adapt the custom Vulkan renderer to consume ECS data.
*   **Status:** input handling. Adapt the custom Vulkan renderer to consume ECS data.
*   **Status:** **Partially Complete**
*    **Complete**
*   **Summary:**
    *   Integrated `bevy_input::InputPlugin`.
    *   Defined Bevy components (`ShapeData`, `Visibility`, `Interaction`) and events (`EntityClicked`, `EntityDragged`, `Hotkey**Summary & Progress:**
    *   **Bevy Input Integration:** `InputPlugin` added. Custom `InteractionController` removed. (**Complete**)
    *   **Core Components Defined:** `ShapeData`, `Visibility`, `Interaction` components created.ActionTriggered`).
    *   Setup spawns entities with Bevy components (`Transform`, custom components). `HotkeyConfig `bevy_transform::components::Transform` used. (**Complete**)
    *   **Core Events Defined:** `Entity` loaded into `HotkeyResource`.
    *   Input/logic handled by Bevy systems (`interaction_system`, `movementClicked`, `EntityDragged`, `HotkeyActionTriggered` events created and registered. Custom `EventBus` removed. (**Complete_system`, `hotkey_system`, `app_control_system`).
    *   Rendering path refactored: `**)
    *   **Setup Refactored:** `Startup` system (`setup_scene_ecs`) spawnsrendering_system` queries ECS, collects `RenderCommandData`. `Renderer::render` accepts `RenderCommandData`. ` entities with Bevy components. `HotkeyConfig` loaded into `HotkeyResource`. Old `Scene` resource removed. (**Complete**)
    *BufferManager` prepares Vulkan resources based on `RenderCommandData`, returning `PreparedDrawData`. `command_buffers` uses `Prepared   **Input/Logic Systems Created:** `interaction_system`, `hotkey_system`, `movement_system`, `app_control_system`DrawData`.
    *   Obsolete legacy code (Scene, EventBus, InteractionController, Renderable, etc.) removed.
    *    implemented using Bevy resources, events, and queries. Old `InteractionController` and `ClickRouter` removed. (**Complete**)
    *   **Follow-up Fixes Implemented:** Pipeline/Shader caching in `BufferManager`. Corrected Vulkan synchronization (fences, command**Rendering Path Refactored:** `rendering_system` queries ECS and collects `RenderCommandData`. `Renderer`, `BufferManager`, `command_buffers` APIs adapted to use `RenderCommandData` and `PreparedDrawData`. (**Complete**)
    *   **Obsolete Code Removed:** Old `Scene`, `EventBus`, `InteractionController`, `Render pool reset, descriptor updates). Fixed swapchain/resize handling (using actual extent). Corrected shader inputs and matching pipeline state. Improved shutdown stability.
*   **Remaining Optimizations/Features:**
    *   Implement GPU resource removal inable` modules/files deleted. (**Complete**)
    *   **Visual Output Debugged:** Rendering output reflects ECS state `BufferManager` for despawned entities.
    *   Implement vertex buffer updates in `BufferManager` based on `Changed. Shader inputs and pipeline state corrected. (**Complete**)
    *   **Resource Caching Implemented:** `Buffer<ShapeData>`.
    *   Implement GPU instancing within the custom Vulkan renderer, driven by ECS data.

### TaskManager` caches Vulkan `Pipeline` and `ShaderModule` resources. (**Complete**)
    *   **Synchronization Fixed:** Fence 6.4: Integrate Logging & Reflection

*   **Goal:** Integrate `bevy_log` for consistent logging and ` usage, command pool reset, descriptor set updates corrected. Resize handling fixed. Shutdown stability improved. (**Complete**)
    *   bevy_reflect` for potential future use (editors, serialization).
*   **Status:** **Partially Complete (Logging Done)**
*   **Summary:** `LogPlugin` added and `bevy_log` macros (`info!`, `warn!**Resource Removal:** Logic to clean up GPU resources for despawned entities is **Not Started**.
    *   **Vertex Updates`, `error!`) used throughout the codebase. Reflection integration (`#[derive(Reflect)]`, registration) is **not yet:** Logic to update vertex buffers on `Changed<ShapeData>` is **Not Started**.
    *   **Inst implemented**.

## Task 7: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layoutancing:** GPU instancing support is **Not Started**.
*   **Outcome:** Core logic runs within ECS. Custom/shaping and implement a custom Vulkan bitmap glyph atlas renderer integrated with the existing Vulkan backend. Display static sample text represented as Bevy ECS entities scene/event management removed. Input uses `bevy_input`. Rendering path consumes ECS data via the custom Vulkan renderer.
- **Status**: Not started
- **Affected Components/Systems/Resources**: *(details omitted for brevity)*
- **Steps, creating necessary GPU resources with caching and correct synchronization. **Key optimizations (resource removal, vertex updates) are still needed.**

### Task 6.**: *(details omitted for brevity)*
- **Constraints**: Requires Task 6 completion. Initial focus on non-wrapping, static text. Rendering must4: Integrate Logging & Reflection

*   **Goal:** Integrate `bevy_log` for consistent logging and ` use the custom Vulkan backend.

## Task 8: Text Handling - Editing & Interaction
- **Goal**:bevy_reflect` for potential future use (editors, serialization).
*   **Status:** **Partially Complete** Integrate `yrs` (`YText`) for collaborative data storage. Implement basic mouse/keyboard editing for text entities using Bevy Input and Systems.
- **Status**: Not started
- **Affected Components/Systems/Resources**: *(details
*   **Summary & Progress:**
    *   **Logging:** `LogPlugin` added. `println!`/`eprintln!` omitted for brevity)*
- **Steps**: *(details omitted for brevity)*
- **Constraints**: Requires Task 7. replaced with `bevy_log` macros (`info!`, `warn!`, `error!`). (**Complete**)
    *   ** Focus on basic local editing. P2P synchronization deferred.

## Task 9: Implement Radial Pie Context Menu
- **Goal**:Reflection:** `bevy_reflect` dependency exists, but components/resources/events are not yet derived from `Reflect` or registered with the `AppTypeRegistry`. (**Not Started**)
*   **Outcome:** Logging is standardized via `bevy_log`. Reflection Implement a popup pie-style context menu triggered by a hotkey, using Bevy ECS entities for UI elements and Bevy events/resources for state management.
- **Status**: Not started
- **Affected Components/Systems/Resources**: *(details omitted for brevity)*
- **Steps**: *(details omitted for brevity)*
- **Constraints**: Requires Task 6 and 7. capabilities are not yet enabled.

## Task 7: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layout/shaping and implement a custom Vulkan bitmap glyph atlas renderer integrated with the existing Vul UI rendering must use the custom Vulkan backend.

## Task 10: Implement Optional Divider System in Framework
- **Goalkan backend. Display static sample text represented as Bevy ECS entities.
- **Status**: **Not Started**
- **Affected Components/Systems**: Implement an optional divider system *within* the framework for managing resizable layout regions using Bevy ECS entities, components, and systems.
- **Status**: Not started
- **Affected Components/Systems/Resources**: *(details omitted for brevity)*
- **/Resources**: (Details remain the same)
- **Steps**: (Details remain the same)
- **Constraints**: RequiresSteps**: *(details omitted for brevity)*
- **Constraints**: Requires Task 6. Rendering content within regions is the application's responsibility Task 6 completion. Initial focus on non-wrapping, static text. Rendering must use the custom Vulkan backend.

.

## Task 11: Enhance Prompt Tool - Code Signature Stripping
- *(No changes needed, this task is external## Task 8: Text Handling - Editing & Interaction
- **Goal**: Integrate `yrs` (`YText`) for collaborative data to the Rust codebase architecture)*
- **Goal**: Add a new option to `utilities/llm_prompt_tool. storage. Implement basic mouse/keyboard editing for text entities using Bevy Input and Systems.
- **Status**: **Not Started**
-sh` that processes Rust files, stripping out function bodies while retaining signatures, `impl` blocks, structs, comments, and other surrounding code.
- **Affected Modules**: `utilities/llm_prompt_tool.sh`.
- **Status**: Not started
- **Steps**: *(remain the same)*
- **Constraints**: *(remain the same)*

## Deferred **Affected Components/Systems/Resources**: (Details remain the same)
- **Steps**: (Details remain the same)
- **Constraints**: Requires Task 7. Focus on basic local editing. P2P synchronization deferred.

## Task 9: Implement Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey, using Bevy: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom), ensuring integration uses ECS entities for UI elements and Bevy events/resources for state management.
- **Status**: **Not Started**
- **Affected Components/Systems/Resources**: (Details remain the same)
- **Steps**: (Details remain the same)
- ** the *custom Vulkan backend*.
- Rendering: Optimizing *custom Vulkan* buffer updates using Bevy changeConstraints**: Requires Task 6 and 7. UI element appearance defined by standard components. Rendering must use the custom Vulkan backend.

 detection (`Changed<T>`). (See Task 6.3 Remaining Optimizations)
- Rendering: Advanced inst## Task 10: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the framework for managing resancing techniques within ECS, implemented *within the custom Vulkan renderer*. (See Task 6.3 Remaining Optimizations)
- Collaborationizable layout regions using Bevy ECS entities, components, and systems.
- **Status**: **Not Started**
- **Affected Components/Systems/Resources**: (Details remain the same)
- **Steps**: (Details remain the same)
- **Constraints**:: Full P2P synchronization of Components/Resources using `yrs` or other CRDTs, integrating with `YrsDocResource`.
- UI: Context switching (using Bevy `States<T>`), undo/redo system (potentially Requires Task 6. Handles layout bounds; rendering content within regions is the application's responsibility.

## Task 11: Enhance Prompt Tool - Code Signature Stripping
- *(No changes needed, this task is external to the Rust codebase architecture via command pattern integrated with ECS).
- General: Performance optimizations leveraging ECS parallelism and queries.
- General: GPU resource cleanup for despawned entities. (See Task 6.3 Remaining Optimizations)