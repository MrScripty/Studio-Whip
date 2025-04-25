# Tasks for `rusty_whip` GUI Framework Enhancements
*Updated: March 19, 2025*

## Overview
These tasks enhance `gui_framework` towards a modular, reusable Bevy plugin structure, supporting future features like advanced UI elements and collaborative editing. The framework uses Bevy's core non-rendering features (ECS, input, events, etc.) and drives a custom Vulkan rendering backend.

**Current Status:** The migration to Bevy (v0.15) is complete (Tasks 6.1-6.4). Core logic resides in Bevy systems within `main.rs`. Rendering uses the custom Vulkan backend, handles dynamic resizing (including background quad vertices), uses a Y-up world coordinate system with correct projection matrix (Y-flip), and supports dynamic vertex buffer updates via `Changed<ShapeData>`. The next major step is refactoring the framework logic currently in `main.rs` into dedicated Bevy plugins.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete** (Legacy - Pre-Bevy Migration)
- **Summary**: Implemented `EventBus`, converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`. Functionality superseded by Bevy Events in Task 6.3.

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Status**: **Complete** (Legacy - Pre-Bevy Migration)
- **Summary**: Created `group.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`. Functionality superseded by Bevy ECS queries/components.

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Status**: **Complete** (Legacy - Pre-Bevy Migration)
- **Summary**: Added `visible` state flag to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` variant to `BusEvent`. Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` to handle `FieldUpdated`. Functionality superseded by Bevy ECS queries and component updates.

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**: Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects.
- **Status**: **Complete** (Legacy - Pre-Bevy Migration)
- **Summary**: Added `visible: bool` field to `Renderable`. Updated `BufferManager::new` to copy visibility state. Modified `record_command_buffers` to check visibility. Functionality superseded by `Visibility` component and `rendering_system` query in Task 6.3.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file (`user/hotkeys.toml`) to map keys/modifiers to action strings, gracefully handling undefined hotkeys. Use `Escape` key for closing the window via event bus and proxy.
- **Status**: **Complete** (Partially Superseded by Task 6.3)
- **Summary**: Added `toml` and `thiserror` dependencies. Created `hotkeys.rs` for config loading/parsing (`HotkeyConfig`, `HotkeyError`) and key formatting. Updated `InteractionController` to load config and publish `BusEvent::HotkeyPressed`. Updated `build.rs` to copy config. Updated `main.rs` to use `EventLoopProxy` and `HotkeyActionHandler` for exit. Hotkey loading (`HotkeyConfig`, `HotkeyResource`) and config file copying remain relevant, but input detection and event dispatch moved to `hotkey_system` using Bevy Input/Events in Task 6.3.

## Task 5: Add Generic Click Handling via Event Router
- **Goal**: Implement a generic mechanism to handle mouse clicks on any `RenderObject` by publishing an `ObjectClicked` event and providing an easy way for the application to register specific callback functions for different object IDs using a central router.
- **Status**: **Complete** (Legacy - Pre-Bevy Migration)
- **Summary**: Added `ObjectClicked` to `BusEvent`. Modified `InteractionController` to publish `ObjectClicked`. Defined `ClickRouter` in `main.rs` implementing `EventHandler` and subscribed it to `EventBus`. Added `register_click_handler` method. Functionality superseded by `EntityClicked` event and standard Bevy event handling in Task 6.3.

## Task 6: Incremental Migration to Bevy Ecosystem

**Overall Goal:** Gradually replace custom framework components (windowing, input, scene management, event bus, math) with Bevy equivalents (`bevy_app`, `bevy_winit`, `bevy_input`, `bevy_ecs`, `bevy_math`, `bevy_log`, `bevy_reflect`, `bevy_utils`), keeping the custom Vulkan renderer. **Strictly avoid `bevy_render`**.

**Status:** **Complete** (Tasks 6.1 - 6.4)

### Task 6.1: Integrate Bevy App & Windowing
*   **Status:** **Complete**
*   **Outcome:** App runs using `bevy_app` runner and `bevy_winit`. Old logic temporarily bridged via Resources.

### Task 6.2: Adopt Bevy Math
*   **Status:** **Complete**
*   **Outcome:** Codebase uses `bevy_math` types (`Vec2`, `Mat4`).

### Task 6.3: Migrate Core Logic to Bevy ECS & Input
*   **Status:** **Complete**
*   **Outcome:** Core logic runs within ECS (`ShapeData`, `Visibility`, `Interaction`, `Transform`). Custom scene/event management removed. Input uses `bevy_input`. Rendering consumes ECS data via the custom Vulkan path, including **dynamic vertex buffer updates** based on `Changed<ShapeData>`. Coordinate systems (Y-up world, Y-flip projection) corrected. Application-specific logic like `background_resize_system` added to `main.rs`.

### Task 6.4: Integrate Logging & Reflection
*   **Status:** **Complete**
*   **Outcome:** Logging standardized via `bevy_log`. Core data types are reflectable.

## Task 7: Refactor Framework into Bevy Plugins
- **Goal**: Refactor framework logic previously residing in `main.rs` into dedicated, modular Bevy plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, `GuiFrameworkDefaultMovementPlugin`, `GuiFrameworkDefaultBindingsPlugin`) for improved encapsulation, reusability, and a cleaner application entry point (`main.rs`).
- **Status**: **Complete**
- **Affected Modules**: `main.rs`, `src/gui_framework/mod.rs`, `src/gui_framework/plugins/mod.rs`, `core.rs`, `interaction.rs`, `movement.rs`, `bindings.rs`, `lib.rs`, `documentation/usage.md`, `documentation/architecture.md`, `documentation/modules.md`.
- **Steps**:
    1.  **Phase 1: Establish Core Plugin:** **Complete.** Created `plugins/core.rs`. Moved Vulkan/Renderer setup, rendering, cleanup, and resize handling systems into `GuiFrameworkCorePlugin`. Registered core types. Ensured `RendererResource` insertion.
    2.  **Phase 2: Establish Interaction Plugin:** **Complete.** Created `plugins/interaction.rs`. Moved hotkey loading (extracted into `load_hotkeys_system`), input processing, and window close request handling systems into `GuiFrameworkInteractionPlugin`. Registered interaction types/events. Ensured `HotkeyResource` insertion.
    3.  **Phase 3: Establish Default Behavior Plugins:** **Complete.** Created `plugins/movement.rs` and `plugins/bindings.rs`. Moved `movement_system` into `GuiFrameworkDefaultMovementPlugin`. Moved `app_control_system` into `GuiFrameworkDefaultBindingsPlugin`.
    4.  **Phase 4: Refinement (System Sets & Cleanup):** **Complete.** Implemented Bevy `SystemSet`s (`CoreSet`, `InteractionSet`, `MovementSet`, `BindingsSet`) within the plugins to manage internal execution order and dependencies. Ensured application systems in `main.rs` (`setup_scene_ecs`) correctly order themselves relative to framework sets using `.after()`. Corrected cleanup system scheduling (`Last` schedule). Cleaned up `main.rs`.
    5.  **Phase 5: Documentation:** **Complete.** Created `usage.md`. Updated `architecture.md` and `modules.md` to reflect the new plugin-based structure.
- **Constraints**: Relied heavily on Bevy's Plugin and System Set mechanisms. Required careful dependency management and scheduling adjustments.

## Task 8: Text Handling - Layout and Rendering Foundation
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
- **Constraints**: Requires Task 7 (Plugin Refactor) completion. Initial focus on non-wrapping, static text. Rendering must use the custom Vulkan backend.

## Task 9: Text Handling - Editing & Interaction
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
- **Constraints**: Requires Task 8. Focus on basic local editing. P2P synchronization deferred.

## Task 10: Implement Radial Pie Context Menu
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
- **Constraints**: Requires Task 7 and 8. UI rendering uses custom Vulkan backend.

## Task 11: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the framework for managing resizable layout regions using Bevy ECS entities, components, and systems. Potentially delivered as another optional plugin.
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
- **Constraints**: Requires Task 7. Handles layout bounds; rendering content within regions is the application's responsibility.

## Task 12: Enhance Prompt Tool - Code Signature Stripping
- *(No changes needed, this task is external to the Rust codebase architecture)*
- **Goal**: Add a new option to `utilities/llm_prompt_tool.sh` that processes Rust files, stripping out function bodies while retaining signatures, `impl` blocks, structs, comments, and other surrounding code.
- **Affected Modules**: `utilities/llm_prompt_tool.sh`.
- **Status**: Not started
- **Steps**: *(remain the same)*
- **Constraints**: *(remain the same)*

## Deferred Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom), ensuring integration uses the *custom Vulkan backend*.
- Rendering: Resource removal for despawned entities (using `RemovedComponents`).
- Rendering: Advanced instancing techniques within ECS, implemented *within the custom Vulkan renderer*.
- Collaboration: Full P2P synchronization of Components/Resources using `yrs` or other CRDTs.
- UI: Context switching (using Bevy `States<T>`), undo/redo system.
- General: Performance optimizations leveraging ECS parallelism and queries.