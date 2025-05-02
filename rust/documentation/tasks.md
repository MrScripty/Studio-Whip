# Tasks for `rusty_whip` GUI Framework Enhancements

## Overview
These tasks enhance `gui_framework` towards a modular, reusable Bevy plugin structure, supporting future features like advanced UI elements and collaborative editing. The framework uses Bevy's core non-rendering features (ECS, input, events, etc.) and drives a custom Vulkan rendering backend.

**Current Status:** The migration to Bevy (v0.15) and the refactor into Bevy plugins (Tasks 6 & 7) are complete. Core logic resides in Bevy systems within framework plugins (`GuiFrameworkCorePlugin`, etc.). Rendering uses the custom Vulkan backend, handles dynamic resizing, uses a Y-up world coordinate system, and supports dynamic vertex buffer updates for shapes. **Task 8 (Text Rendering Foundation) is complete.** Text layout occurs via `cosmic-text`, glyphs are cached/uploaded to a Vulkan atlas, vertices are generated, and text is drawn using a dedicated pipeline. **Task 9 (Text Editing) is in progress:** Yrs integration, focus management, and layout caching (`TextBufferCache`) are complete. Cursor state components (`CursorState`, `CursorVisual`) have been added.

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
- **Status**: **Complete**
- **Affected Components/Systems/Resources**:
    - New Component: `Text { content: String, size: f32, color: Color, alignment: TextAlignment, bounds: Option<Vec2> }` (`#[derive(Component, Reflect)]`) - **Implemented**
    - New Component: `TextLayoutOutput { glyphs: Vec<PositionedGlyph> }` (`#[derive(Component)]`) - **Implemented** (Not Reflectable)
    - New Struct: `PositionedGlyph { glyph_info: GlyphInfo, layout_glyph: LayoutGlyph, vertices: [Vec2; 4] }` - **Implemented** (Not Reflectable)
    - New Struct: `GlyphInfo { pixel_x: u32, pixel_y: u32, pixel_width: u32, pixel_height: u32, uv_min: [f32; 2], uv_max: [f32; 2] }` (`#[derive(Reflect)]`) - **Implemented**
    - New Resource: `GlyphAtlasResource(Arc<Mutex<GlyphAtlas>>)` - **Implemented** (Manages Vulkan Image/View/Sampler)
    - New Resource: `FontServerResource(Arc<Mutex<FontServer>>)` - **Implemented** (Manages `cosmic_text::FontSystem` and `fontdb::Database`)
    - New Resource: `SwashCacheResource(Mutex<SwashCache>)` - **Implemented**
    - New Systems: `text_layout_system` - **Implemented**
    - Modified Systems: `rendering_system` (queries updated, **generates text vertices**, calls `Renderer::render` with text data), `create_*` systems in `core.rs` (added atlas, font server, swash cache init), `cleanup_trigger_system` (added atlas cleanup).
    - Modified Renderer (`render_engine.rs`): Manages text pipeline, dynamic text vertex buffer, atlas descriptor set.
    - Modified Command Buffers (`command_buffers.rs`): Records text draw calls.
- **Steps**:
    1.  **Add Dependencies:** Add `cosmic-text`, `fontdb`, `swash`, `rectangle-pack`, `bevy_color` to `Cargo.toml`. - **Complete.**
    2.  **Define `Text` Component:** Create the `Text` component struct (`text_data.rs`) to hold text data. - **Complete.**
    3.  **Implement `GlyphAtlas` Logic:** Create a module/struct (`glyph_atlas.rs`) responsible for managing a Vulkan texture atlas. - **Complete.**
        *   Initialize the Vulkan `Image`, `ImageView`, `Sampler`. - **Complete.**
        *   Manage state via `GlyphAtlasResource`. - **Complete.**
        *   Use `rectangle-pack` to find space for new glyphs (persistent packing state). - **Implemented.**
        *   Use `swash` image data as input for glyph dimensions and bitmap. - **Implemented.**
        *   Upload glyph bitmaps to the Vulkan texture (using staging buffers). - **Implemented** (via `upload_glyph_bitmap`).
        *   Store glyph UV coordinates (`GlyphInfo`). - **Implemented.**
    4.  **Implement `FontServer` Resource:** Create a resource (`font_server.rs`) to load and manage fonts using `cosmic_text::FontSystem` and `fontdb`. Load system fonts at startup. - **Complete.**
    5.  **Create `text_layout_system`:** - **Complete.**
        *   Queries for entities with `(Changed<Text>, &Transform, With<Visibility>)`.
        *   Uses `FontServerResource`, `GlyphAtlasResource`, `SwashCacheResource`.
        *   Uses `cosmic_text::Buffer` for layout/shaping.
        *   Calculates baseline-aligned vertex positions using `run.line_y`, `layout_glyph.y`, and `swash_image.placement`.
        *   Calls `GlyphAtlas::add_glyph` (which now handles packing/upload).
        *   Stores layout results (`PositionedGlyph`) in `TextLayoutOutput` component.
    6.  **Implement Text Rendering Pipeline:** - **Complete.** (Integrated into `rendering_system` and `Renderer`)
        *   `rendering_system` queries `TextLayoutOutput`.
        *   `rendering_system` generates world-space `TextVertex` data (with flipped V coordinates for UV mapping).
        *   `Renderer` manages dynamic Vulkan vertex buffer for text (creation, resize, update).
        *   `Renderer` manages Vulkan descriptor set for glyph atlas.
        *   `Renderer` creates and manages text graphics pipeline.
        *   `Renderer::render` calls `record_command_buffers` with text data.
        *   `record_command_buffers` binds text pipeline, vertex buffer, descriptor sets, and issues draw calls.
    7.  **Integrate into App:** - **Complete.** (`Text` component, `FontServerResource`, `GlyphAtlasResource`, `SwashCacheResource`, `text_layout_system`, text rendering logic in `rendering_system`/`Renderer`/`command_buffers`, text shaders, Vulkan layouts/pipeline all integrated). Sample text added in `main.rs`.
    8.  **Test:** - **Complete.** (Basic text renders with correct baseline alignment. Minor artifacts with linear filtering noted).
- **Constraints**: Requires Task 7 (Plugin Refactor) completion. Initial focus on non-wrapping, static text. Rendering uses the custom Vulkan backend. **Known issues:** Text descriptor Set 0 binding uses an incorrect workaround. Text rendering resource management could be refactored for efficiency/encapsulation. Minor visual artifacts may occur near glyph edges with linear filtering.

## Task 9: Text Handling - Editing, Selection, Highlighting & Clipboard
- **Goal**: Integrate `yrs` for data storage. Implement mouse/keyboard editing for `EditableText` entities, including cursor management, text selection (click, drag, double-click, keyboard), visual highlighting, and clipboard (cut/copy/paste) integration.
- **Status**: **In Progress** (Yrs integration, hit detection, focus management, basic cache/state components complete)
- **Affected Components/Systems/Resources**:
    - Modified Component: `Text` (removed `content`).
    - New Components: `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextBufferCache`, `TextSelection { start: usize, end: usize }`, `HighlightVisual` (marker).
    - New Resources: `YrsDocResource`, `MouseContext { context: Option<MouseContextType> }`, `ClipboardResource` (e.g., holding `arboard::Clipboard`). Potentially `UndoRedoHistory`.
    - New Systems: `manage_cursor_visual_system`, `update_cursor_transform_system`, `text_editing_system` (expanded), `TextDragSystem`, `highlight_selection_system`, clipboard handling systems/logic.
    - Modified Systems: `interaction_system` (context, click/double-click logic, shift+click), `text_layout_system` (reads Yrs, writes cache), `cleanup_trigger_system` (cleans cache).
    - New Events: `TextFocusChanged`, `YrsTextChanged`.
    - New Utility: `get_cursor_at_position` function.
    - New Shaders: `cursor.vert`/`.frag`, `highlight.vert`/`.frag`.
- **Steps (Phased Implementation with Integrated Testing)**:
    1.  **Phase 1: Cursor Foundation**
        *   Implement `manage_cursor_visual_system` (spawns/despawns `CursorVisual`, adds/removes `CursorState`).
        *   Implement `update_cursor_transform_system` (positions `CursorVisual` via `layout_cursor`).
        *   Create `cursor.vert`/`.frag` shaders, update `build.rs`.
        *   **TEST:** Cursor appears/disappears with focus, positions correctly.
    2.  **Phase 2: Basic Selection State & Context**
        *   Define `TextSelection` component, `MouseContext` resource. Register them.
        *   Create `get_cursor_at_position` utility function (using `buffer.hit`).
        *   Modify `interaction_system`: Set `MouseContext` on mousedown. Handle basic click on `EditableText` (set `CursorState`, set `TextSelection` with start=end=click_pos, clear previous selection).
        *   Modify `manage_cursor_visual_system`: Hide cursor if `selection.start != selection.end`.
        *   **TEST:** Context set correctly. Clicking sets cursor/selection. Cursor hides when selection active. Non-text drag works.
    3.  **Phase 3: Refined Drag Selection & Clearing**
        *   Create `TextDragSystem`: Handle mouse drag when `MouseContext::Text`. Use `get_cursor_at_position` to detect character position changes (drag threshold). Update `TextSelection.end`. Keep selection active on release.
        *   Modify `interaction_system`: Implement second-click logic (if click on text entity that already has active selection, clear selection and place cursor).
        *   **TEST:** Drag selection works (character threshold, multi-line). Second-click clears selection correctly.
    4.  **Phase 4: Selection Editing & Extension**
        *   Modify `text_editing_system`: Handle Backspace/Delete/Typing when `TextSelection` is active (replace/delete range). Implement Shift+Arrow key logic to modify `TextSelection` range based on `cursor_motion`.
        *   Modify `interaction_system`: Implement Shift+Click logic to set `TextSelection` range (from current `CursorState.position` to click position).
        *   **TEST:** Deleting/typing over selections. Extending selections with Shift+Arrow/Shift+Click.
    5.  **Phase 5: Double-Click Word Selection**
        *   Modify `interaction_system`: Detect double-clicks on `EditableText`.
        *   Implement word boundary detection logic around the click index.
        *   Update `TextSelection` to select the word. Update `CursorState.position`.
        *   **TEST:** Double-click selects words correctly (including punctuation/whitespace boundaries).
    6.  **Phase 6: Visual Feedback (Highlighting)**
        *   Implement `highlight_selection_system` triggered by `Changed<TextSelection>` or `YrsTextChanged`.
        *   Calculate highlight rectangles (potentially multi-line) using `TextBufferCache` and `cosmic-text` layout info.
        *   Spawn/update `HighlightVisual` entities (using `ShapeData` with `highlight.vert`/`.frag`). Render behind text (Z offset).
        *   Create `highlight.vert`/`.frag` shaders, update `build.rs`.
        *   **TEST:** Highlights appear correctly for all selection methods, update dynamically, handle multi-line, render behind text.
    7.  **Phase 7: Clipboard Integration (Cut/Copy/Paste)**
        *   Add clipboard dependency (`arboard`). Create/initialize `ClipboardResource`.
        *   Implement Cut/Copy/Paste logic (likely via hotkey handlers reacting to `HotkeyActionTriggered`).
        *   Interact with `ClipboardResource`, `YrsDocResource`, `TextSelection`, `CursorState`.
        *   Send `YrsTextChanged` on Cut/Paste.
        *   **TEST:** Cut/Copy/Paste within editor and with external applications. Test with/without active selections.
    8.  **Phase 8: Undo/Redo Foundation (Potentially Deferred)**
        *   Define `UndoRedoHistory` resource.
        *   Modify editing actions (Type, Delete, Cut, Paste) to use `doc.transact_mut_with_origin(...)` and store origins.
        *   Implement basic Undo/Redo hotkey handlers calling `doc.undo()`/`doc.redo()`.
        *   **TEST (Basic):** Simple undo/redo works. (Acknowledge cursor/selection state restoration is complex and likely incomplete in this phase).
    9.  **Phase 9: Optimization (Post-Functionality)**
        *   Implement optimizations (e.g., highlight batching, cursor calculation caching, event-driven system execution).
        *   **TEST:** Profile and verify performance improvements and lack of regressions.
- **Constraints**: Requires Task 8 completion. Requires `yrs`, `cosmic-text`, `arboard`. Requires careful handling of coordinate systems, UTF-8 byte offsets, Yrs transactions, and Bevy system ordering/dependencies. Full Undo/Redo state restoration is complex. P2P synchronization deferred.

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
    4.  **Rendering:** Existing `rendering_system` (using custom Vulkan backend) will render the spawned UI entities based on their standard components (`Transform`, `ShapeData`, `Text`, `Visibility`).
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