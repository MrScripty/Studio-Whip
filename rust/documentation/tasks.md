# Tasks for `rusty_whip` GUI Framework Enhancements (March 19, 2025 - Updated)

## Overview
These tasks enhance `gui_framework` to support a future divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building specific UIs atop it. Recent focus: Implementing event bus, refactoring rendering, logical grouping, and batch updates via events.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete**
- **Summary**: Implemented `EventBus`, converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`.

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`, `src/gui_framework/mod.rs`.
- **Status**: **Complete** (Steps 1-4 done; Step 5 moved to Task 3; Step 6 moved to Task 3.1).
- **Summary**: Created `group.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`.

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Affected Modules**: `src/gui_framework/scene/group.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/event_bus.rs`, `src/gui_framework/window/window_handler.rs`.
- **Status**: **Complete**
- **Summary**: Added `visible` state flag to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` variant to `BusEvent` (using `Arc<dyn Any + Send + Sync>`). Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` to handle `FieldUpdated` and modify `RenderObject` state. Tested in `main.rs`.
- **Notes**: This implements batch *triggering*, not a single batch processing event. Changing `depth` requires renderer re-sorting (not implemented). Changing `visible` requires renderer modification (Task 3.1).

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**: Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects. (Deferred from Task 2, Step 6).
- **Affected Modules**: `src/gui_framework/rendering/render_engine.rs` or `src/gui_framework/rendering/command_buffers.rs`.
- **Status**: Not started
- **Steps**:
    1. Modify `Renderer::render` or `record_command_buffers` to check the visibility state (likely from the corresponding `Renderable` which should mirror `RenderObject.visible`) before binding/drawing.
    2. Test: Use `set_field("visible", false)` from Task 3 in `main.rs` and verify that the corresponding objects are no longer drawn.
- **Constraints**: Requires access to the visibility state during rendering command generation.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file to map keys to events, gracefully handling undefined hotkeys. Use `Ctrl+Esc` for closing the window.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, New `src/gui_framework/input/hotkeys.rs`, `src/gui_framework/mod.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1. Add `toml` dependency to `Cargo.toml`.
    2. Create `hotkeys.rs` with `HotkeyConfig` struct and `load_config` function. Define `HotkeyError`.
    3. Update `InteractionController` to load config and emit `BusEvent::HotkeyPressed(action: Option<String>)` on keyboard input matching a defined hotkey (incl. handling `Ctrl+Esc` to perhaps emit a specific "CloseRequested" action string).
    4. Handle `HotkeyPressed` events in a subscriber (e.g., `main.rs` or `window_handler.rs`). The handler for the "CloseRequested" action would trigger `event_loop.exit()`. Other actions log "No action" or perform defined tasks.
    5. Integrate `hotkeys.rs` into `gui_framework/mod.rs`.
    6. Test with `hotkeys.toml` including the `Ctrl+Esc` mapping.
- **Constraints**: Use `toml` crate. Relies on `EventBus`.

## Task 5: Add Button Functionality
- **Goal**: Extend `gui_framework` to support button behavior on any `RenderObject` via events, using a state flag.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1. Update `RenderObject`: Add `is_button: bool` state flag and `action_id: Option<String>`. Update instantiations.
    2. Update `InteractionController::handle_event`: On receiving `ObjectPicked`, check if the corresponding `RenderObject.is_button` state flag is true. If yes, publish `BusEvent::ButtonClicked(action_id: Option<String>)`.
    3. Test: Add a button object in `main.rs`, subscribe to `ButtonClicked` in `main.rs`, verify log output on click.
- **Constraints**: Event-driven; relies on state flag; rendering unchanged.

## Task 6: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layout/shaping and implement a custom Vulkan bitmap glyph atlas renderer. Display static sample text (English/Chinese placeholder).
- **Affected Modules**: New `src/gui_framework/rendering/text_renderer/mod.rs`, `src/gui_framework/rendering/text_renderer/glyph_atlas.rs`, `src/gui_framework/rendering/buffer_manager.rs` (integration), `src/gui_framework/rendering/render_engine.rs` (integration), `src/gui_framework/scene/scene.rs` (add text objects), `src/main.rs` (testing), New shaders (`glyph.vert`, `glyph.frag`).
- **Status**: Not started
- **Steps**:
    1. Add dependencies: `cosmic-text`, `fontdb`, `swash` (likely transitive via cosmic-text), potentially `rectangle-pack`.
    2. Create `glyph_atlas.rs` module to manage Vulkan `vk::Image` atlas, handle glyph rasterization requests via `swash`, manage packing, upload bitmaps to GPU, and track UV coordinates.
    3. Create `text_renderer.rs` module to:
        *   Hold `cosmic_text::FontSystem`, `SwashCache`.
        *   Provide an API to add/update text buffers (`cosmic_text::Buffer`).
        *   Trigger `cosmic-text` shaping/layout.
        *   Interact with `GlyphAtlasManager` to ensure glyphs are rasterized and in the atlas.
        *   Generate vertex data (quads) for visible glyphs based on layout results and atlas UVs.
    4. Integrate the text renderer into the main rendering loop (`render_engine.rs` / `buffer_manager.rs`):
        *   Initialize/cleanup text rendering resources.
        *   Manage dynamic vertex buffers for glyph quads.
        *   Create and manage Vulkan pipeline/shaders for glyph rendering.
        *   Bind atlas texture and draw glyph quads.
    5. Modify `RenderObject` or create a new `TextObject` struct in `scene.rs` to hold text data.
    6. Test in `main.rs`: Create text objects with sample English and Chinese text, verify they are rendered correctly.
- **Constraints**: Focus on bitmap rendering. Requires significant Vulkan integration work. Defer SDF rendering. Supports English now, lays foundation for Chinese.

## Task 7: Text Handling - Editing & Interaction
- **Goal**: Integrate `yrs` (`YText`) for collaborative data storage. Implement basic mouse/keyboard editing for text objects.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs` (manage `YText` instances or refs), `src/gui_framework/rendering/text_renderer/mod.rs` (update from `YText`, render cursor), `src/main.rs` (testing).
- **Status**: Not started
- **Steps**:
    1. Add dependency: `yrs`.
    2. Replace/Augment text storage in `scene.rs` with `yrs::Text` (or references to a shared `YDoc`).
    3. Modify `InteractionController` to:
        *   Track focused text object.
        *   Handle keyboard events (chars, backspace, delete, potentially arrows) by generating `YText` operations (e.g., `ytext.insert`, `ytext.remove_range`).
        *   Handle mouse clicks to calculate text position using `cosmic-text` layout info and set cursor position.
    4. Modify `TextRenderer` / main loop to observe changes in `YText` (likely via callbacks or polling) and trigger re-layout/re-rendering.
    5. Modify `TextRenderer` to draw a cursor based on the current cursor position state.
    6. Test: Create an editable text object, type text, delete text, move cursor with mouse, verify visual updates.
- **Constraints**: Basic editing first. Defer complex selections, advanced cursor movement. Focus on local editing triggering CRDT ops; actual P2P sync is later.

## Task 8: Implement Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey (e.g., Ctrl+Space), with dynamic options.
- **Affected Modules**: New `src/gui_framework/ui/pie_menu.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/rendering/text_renderer/mod.rs` (for labels), New shaders (`pie_menu.vert`, `pie_menu.frag`), `src/gui_framework/event_bus.rs`, `src/gui_framework/mod.rs`.
- **Status**: Not started
- **Steps**:
    1. Create `pie_menu.rs` with `PieMenu` struct (options, geometry) and `PieOption` struct (label, action_id). Implement rendering logic (using standard shapes and Task 6 text rendering for labels).
    2. Update `Scene` or a UI manager: Add `active_menu: Option<PieMenu>`.
    3. Update `InteractionController`: Handle hotkey (Task 4) `BusEvent::HotkeyPressed("show_pie_menu")`. Publish `BusEvent::ShowPieMenu(position: [f32; 2], context: ContextEnum)`.
    4. Update `Scene`/UI manager: Subscribe to `ShowPieMenu`, create `PieMenu` based on context, set `active_menu`.
    5. Update `InteractionController`: Handle clicks when `active_menu` is Some. Determine selected option based on click position relative to menu geometry. Publish `BusEvent::MenuOptionSelected(action_id: String)`. Clear `active_menu`.
    6. Update `Renderer`/`BufferManager`/`TextRenderer`: Draw `active_menu` if it exists.
    7. Test: Trigger menu via hotkey, verify display, select option, verify event.
- **Constraints**: Event-driven; uses Task 6 text rendering. Requires hotkey system (Task 4).

## Task 9: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the `gui_framework` for managing resizable layout regions. Provide an API for end-users to enable and configure it.
- **Affected Modules**: New `src/gui_framework/ui/dividers.rs`, `src/gui_framework/scene/scene.rs` (or new UI manager), `src/gui_framework/interaction/controller.rs` (potential focus/context), `src/gui_framework/mod.rs` (API exposure), `src/main.rs` (testing).
- **Status**: Not started
- **Steps**:
    1.  Define `Divider` struct (visual representation via `RenderObject`, orientation, constraints, associated regions/IDs) and `DividerSystem` struct within `dividers.rs`.
    2.  Integrate `DividerSystem` management into `Scene` (or a new higher-level UI state manager within the framework if `Scene` becomes too cluttered). Provide methods like `Scene::enable_dividers()`, `Scene::add_divider(...)`.
    3.  Use `Scene::add_object` internally to create the visual representation of dividers (likely simple rectangles). Ensure they are marked `is_draggable`.
    4.  Handle `ObjectMoved` events for divider objects within the `DividerSystem` (or a dedicated event handler). Implement logic to:
        *   Constrain divider movement (e.g., within parent bounds, minimum region sizes).
        *   Calculate resulting region size changes based on divider movement.
        *   Publish new events like `RegionResized { region_id: usize, new_rect: Rect }` via the main `EventBus` for the application layer to react to.
    5.  Expose the necessary configuration and `RegionResized` event handling capabilities through the public API of `gui_framework`.
    6.  Test in `main.rs`: Enable the divider system, add a divider, verify rendering, drag the divider, and subscribe to/log `RegionResized` events.
- **Constraints**: Builds on framework primitives. Needs clear API separation between internal management and end-user configuration/events. Layout logic calculates new region dimensions; rendering content within those regions is the application's responsibility.

## Deferred Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom).
- Rendering: Depth-based re-sorting of renderables.
- Collaboration: Full P2P synchronization of `yrs` CRDT operations.
- UI: Context switching, undo/redo system.
- General: Performance optimizations (e.g., instance buffer resizing).