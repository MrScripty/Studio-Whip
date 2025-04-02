# Tasks for `rusty_whip` GUI Framework Enhancements (March 19, 2025)

## Overview
These tasks enhance `gui_framework` to support a divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building the specific divider GUI atop it. Recent focus: Implementing an event bus and refactoring rendering components.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete**
- **Summary**:
    - Created `event_bus.rs` with `EventBus`, `BusEvent`, and `EventHandler`.
    - Modified `InteractionController` to publish `ObjectMoved`/`ObjectPicked` events.
    - Modified `Scene` to publish `InstanceAdded` events.
    - Implemented `SceneEventHandler` (in `window_handler.rs`) to handle `ObjectMoved` and update `Scene` state.
    - Refactored `Renderer` to implement `EventHandler`, handle `InstanceAdded` via an internal queue, and delegate buffer/pipeline management to `BufferManager`/`PipelineManager`.
    - Updated `WindowHandler` to manage `Arc<Mutex<>>` state, subscribe handlers, and ensure correct cleanup order (`EventBus::clear`, `Renderer::cleanup`, `cleanup_vulkan`).
    - Corrected instancing draw calls in `command_buffers.rs`.
- **Affected Modules**: `event_bus.rs`, `interaction/controller.rs`, `scene/scene.rs`, `rendering/*`, `window/window_handler.rs`, `context/*`, `main.rs`, `lib.rs`, `gui_framework/mod.rs`.

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`, `src/gui_framework/mod.rs`.
- **Status**: In progress; Steps 1-4 complete, Step 5 moved to Task 3, Step 6 deferred.
- **Steps**:
    1.  **Clean Up Old Group Logic** (Complete)
    2.  **Create `group.rs` with `Group` and `GroupManager`** (Complete)
    3.  **Add `GroupEditor` to `group.rs`** (Complete)
    4.  **Integrate `GroupManager` into `Scene`** (Complete)
    5.  **Implement Generic Batch Updates** (Moved to Task 3)
    6.  **Optional Renderer Adjustment** (Deferred)
- **Constraints**: Groups are purely logical. Objects can be in multiple groups.
- **Notes**: Foundation for batch operations is laid.

## Task 3: Implement Group Batch Updates via Events
- **Goal**: Add generic batch update functionality to `GroupEditor` for efficient group operations using the event bus.
- **Affected Modules**: `src/gui_framework/scene/group.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1.  **Add Visibility Field**: Add `visible: bool` to `RenderObject` (default `true`).
    2.  **Implement Batch Updates**: Add `GroupEditor::set_field<T>(&mut self, field: &str, value: T) -> Result<(), FieldError>`:
        *   Supports fields like `is_draggable`, `visible`, `offset`, `depth`.
        *   Emits `BusEvent::FieldUpdated(object_id, field_name: String, value: Box<dyn Any + Send + Sync>)` for each object via `EventBus`. (Event definition needs refinement for generic values).
        *   Define `FieldError { InvalidField, TypeMismatch, InvalidObject }`.
    3.  **Handle Events**: Update `Scene` (or a dedicated handler) to subscribe to `FieldUpdated` events, dynamically apply changes to `RenderObject` fields using reflection or matching.
    4.  **Test**: In `main.rs`, create a group, use `set_field` (e.g., set `visible` to `false`), verify event publishing and state updates in `Scene`.
- **Constraints**: Uses event bus; rendering impact depends on field updated (e.g., `visible`). Requires careful handling of generic event data.
- **Notes**: Completes Task 2’s remaining goal; renderer adjustment for visibility (Task 2, Step 6) can be done after this.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file to map keys to events, gracefully handling undefined hotkeys.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, New `src/gui_framework/input/hotkeys.rs`, `src/gui_framework/mod.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1.  Create `hotkeys.rs` with `HotkeyConfig` struct and `load_config` function (using `toml` crate). Define `HotkeyError`.
    2.  Update `InteractionController` to load config and emit `BusEvent::HotkeyPressed(action: Option<String>)` on keyboard input matching a defined hotkey.
    3.  Handle `HotkeyPressed` events in a subscriber (e.g., `main.rs` or dedicated handler) to perform actions or log "No action".
    4.  Integrate `hotkeys.rs` into `gui_framework/mod.rs`.
    5.  Test with `hotkeys.toml`.
- **Constraints**: Use `toml` crate. Relies on `EventBus`.
- **Notes**: Ctrl+Space potentially reserved for Task 7.

## Task 5: Add Button Functionality
- **Goal**: Extend `gui_framework` to support button behavior on any `RenderObject` via events.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1.  Update `RenderObject`: Add `is_button: bool` and `action_id: Option<String>` (using String for flexibility).
    2.  Update `InteractionController::handle_event`: If a clicked object (`ObjectPicked` event source) has `is_button = true`, publish `BusEvent::ButtonClicked(action_id: Option<String>)`.
    3.  Test: Add a button object in `main.rs`, subscribe to `ButtonClicked` in `main.rs`, verify log output on click.
- **Constraints**: Event-driven; rendering unchanged.
- **Notes**: `action_id` triggers events; subscribers define behavior.

## Task 6: Add Text Display with Rendering Module
- **Goal**: Support static text display with an extensible rendering module (placeholder initially).
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/rendering/renderable.rs`, New `src/gui_framework/rendering/text_renderer.rs`, New shaders (`text.vert`, `text.frag`), `src/gui_framework/rendering/mod.rs`, `src/gui_framework/rendering/buffer_manager.rs`.
- **Status**: Not started
- **Steps**:
    1.  Update `RenderObject`: Add `text: Option<String>` and `font_size: f32`.
    2.  Create `text_renderer.rs` with `TextRenderer` struct and placeholder rendering logic (e.g., colored quads).
    3.  Integrate `TextRenderer` into `BufferManager` or `Renderer` to draw text for relevant objects.
    4.  Test: Add a text object in `main.rs`, verify placeholder display.
- **Constraints**: Placeholder rendering initially.
- **Notes**: Prepares for Task 7 text editing. Requires new shaders.

## Task 7: Implement Text Editing
- **Goal**: Add support for editable text fields using events.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/rendering/renderable.rs`, `src/gui_framework/rendering/text_renderer.rs`, `src/gui_framework/event_bus.rs`.
- **Status**: Not started
- **Steps**:
    1.  Track active text field (e.g., `active_text_object_id: Option<usize>` in `InteractionController` or `Scene`).
    2.  Update `InteractionController`: On click (`ObjectPicked`), set active field if object has text. Handle keyboard input (`winit::event::KeyEvent`), publish `BusEvent::TextEdited(object_id: usize, new_text: String)`. Handle focus loss (e.g., click elsewhere) to clear active field.
    3.  Update `Scene` (or handler): Subscribe to `TextEdited`, update `RenderObject::text`.
    4.  Update `TextRenderer`: Render a cursor based on active field state and cursor position (add `cursor_pos: usize` to `RenderObject` or manage in `InteractionController`).
    5.  Test: Click a text field, type, verify updates via events and visual cursor/text changes.
- **Constraints**: Basic ASCII input; defer advanced rendering/layout. Event-driven.
- **Notes**: Builds on Task 6.

## Task 8: Add Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey (e.g., Ctrl+Space), with dynamic options.
- **Affected Modules**: New `src/gui_framework/ui/pie_menu.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/rendering/text_renderer.rs`, New shaders (`pie_menu.vert`, `pie_menu.frag`), `src/gui_framework/event_bus.rs`, `src/gui_framework/mod.rs`.
- **Status**: Not started
- **Steps**:
    1.  Create `pie_menu.rs` with `PieMenu` struct (options, geometry) and `PieOption` struct (label, action_id). Implement rendering logic (torus, text placement using `TextRenderer`).
    2.  Update `Scene` or a UI manager: Add `active_menu: Option<PieMenu>`.
    3.  Update `InteractionController`: Handle hotkey (Task 4) `BusEvent::HotkeyPressed("show_pie_menu")`. Publish `BusEvent::ShowPieMenu(position: [f32; 2], context: ContextEnum)`.
    4.  Update `Scene`/UI manager: Subscribe to `ShowPieMenu`, create `PieMenu` based on context, set `active_menu`.
    5.  Update `InteractionController`: Handle clicks when `active_menu` is Some. Determine selected option based on click position relative to menu geometry. Publish `BusEvent::MenuOptionSelected(action_id: String)`. Clear `active_menu`.
    6.  Update `Renderer`/`BufferManager`: Draw `active_menu` if it exists.
    7.  Test: Trigger menu via hotkey, verify display, select option, verify event.
- **Constraints**: Event-driven; uses Task 6 text rendering. Requires hotkey system (Task 4).
- **Notes**: Prepares for GUI builder interactions.

## Task 9: Initial `gui_app/dividers.rs` Outline
- **Goal**: Sketch a basic divider system for layout management using the enhanced framework API.
- **Affected Modules**: New `src/gui_app/dividers.rs`, `src/main.rs`.
- **Status**: Not started
- **Steps**:
    1.  Define `Divider` struct (position, orientation, associated panels/regions).
    2.  Implement `DividerSystem` to manage `Divider` objects using `Scene::add_object` for visual representation and interaction.
    3.  Handle dragging (`ObjectMoved` events for divider objects) to update layout logic (resize adjacent regions - details TBD).
    4.  Test in `main.rs`: Add a divider, verify rendering and basic dragging via events.
- **Constraints**: Initial sketch; full layout logic deferred. Builds on event-driven framework.
- **Notes**: Connects framework features to a specific application need.

## Dependencies
- Relies on `Scene` for state, `Renderer` for rendering, `EventBus` for communication.

## Future Considerations
- Build interactive GUI layout system post-Task 9.
- Enhance `text_renderer.rs` for advanced features (fonts, markdown, etc.).
- Implement instance buffer resizing in `BufferManager`.