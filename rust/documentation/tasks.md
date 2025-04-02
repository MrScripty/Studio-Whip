# Tasks for `rusty_whip` GUI Framework Enhancements (March 19, 2025)

## Overview
These tasks enhance `gui_framework` to support a divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building the specific divider GUI atop it. Recent focus: refactoring `render_engine.rs` for modularity and preparing for an event-driven architecture.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Affected Modules**:
  - New `src/gui_framework/event_bus.rs`
  - `src/gui_framework/interaction/controller.rs`
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/rendering/render_engine.rs`
  - `src/gui_framework/window/window_handler.rs`
  - `src/gui_framework/mod.rs`
- **Status**: Not started
- **Steps**:
  1. **Create `event_bus.rs`**:
     - Define `enum Event { ObjectMoved(usize, [f32; 2], Option<usize>), InstanceAdded(usize, [f32; 2]), ObjectPicked(usize, Option<usize>), RedrawRequested, ... }`.
     - Define `trait EventHandler { fn handle(&mut self, event: &Event); }`.
     - Implement `struct EventBus { subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>> }` with `subscribe<T: 'static>(&mut self, handler: impl EventHandler + 'static)` and `publish(&self, event: Event)`.
  2. **Integrate Event Bus**:
     - Add `bus: &EventBus` to `VulkanContextHandler` in `window_handler.rs`, initialize in `main.rs`.
     - Pass `bus` to `InteractionController`, `Scene`, and `Renderer`.
  3. **Convert Dragging**:
     - Update `controller.rs` `handle_event` to emit `ObjectMoved(index, [dx, dy], instance_id)` instead of calling `Scene::translate_object`.
     - Make `Scene` implement `EventHandler`, update `RenderObject` offsets in `handle(Event::ObjectMoved)`.
  4. **Convert Instancing**:
     - Update `Scene::add_instance` to emit `InstanceAdded(object_id, offset)`; `Renderer` subscribes to sync `instance_buffer`.
  5. **Convert Rendering Triggers**:
     - `controller.rs` emits `RedrawRequested` on input; `render_engine.rs` subscribes to call `render`.
  6. **Test**:
     - In `main.rs`, drag an object/instance, verify `Scene` updates and `Renderer` redraws via events.
     - Add an instance, confirm `Renderer` syncs without direct calls.
- **Constraints**: Lightweight, single-threaded initially; no new dependencies beyond `std`.
- **Notes**: Replaces direct calls with pub-sub; foundation for P2P and new features.

## Task 2 (In Progress): Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers for batch operations on `RenderObject`s, decoupled from rendering and interaction, supporting multiple group membership per object.
- **Affected Modules**: 
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/scene/group.rs`
  - `src/gui_framework/mod.rs`
- **Status**: In progress; Steps 1-4 complete, Step 5 pending, Step 6 optional
- **Steps**:
  1. **Clean Up Old Group Logic** (Complete):
     - Removed `groups: Vec<Group>` field from `Scene` struct.
     - Removed methods `add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize` and `add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>)` from `scene.rs`.
     - Updated `pick_object_at(&self, x: f32, y: f32) -> Option<(usize, Option<usize>)>` to return only pool indices, removing group ID prioritization.
     - Updated `translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>)` to handle object/instance dragging only, removing group translation logic.
     - Updated `main.rs` to remove old group creation (replaced with individual object adds).
     - Tested: Dragging works on individual objects/instances, no group interference.
  2. **Create `group.rs` with `Group` and `GroupManager`** (Complete):
     - Defined `Group { name: String, object_ids: Vec<usize> }` for named groups with pool indices.
     - Defined `GroupManager { groups: Vec<Group> }` with methods:
       - `new(&mut self, name: &str) -> Result<(), GroupError>`: Creates a unique named group.
       - `delete(&mut self, name: &str) -> Result<(), GroupError>`: Deletes a group.
       - `edit(&mut self, name: &str) -> Result<GroupEditor, GroupError>`: Returns editor for group modifications.
       - `groups_for_object(&self, object_id: usize) -> Vec<&str>`: Lists groups an object belongs to.
     - Defined error enum `GroupError { DuplicateName, GroupNotFound }`.
     - Tested: Create/delete groups, query empty membership.
  3. **Add `GroupEditor` to `group.rs`** (Complete):
     - Defined `GroupEditor<'a> { group: &'a mut Group, scene: &'a mut Scene }` for editing a group with access to `Scene`.
     - Implemented:
       - `add_object(&mut self, object_id: usize)`: Adds an object to the group.
       - `remove_object(&mut self, object_id: usize)`: Removes an object from the group.
     - Tested: Add/remove objects, verified `object_ids` updates.
  4. **Integrate `GroupManager` into `Scene`** (Complete):
     - Added `groups: GroupManager` to `Scene` struct.
     - Updated `Scene::new` to initialize `GroupManager`.
     - Added `Scene::groups(&mut self) -> &mut GroupManager` method.
     - Updated `src/gui_framework/mod.rs` with `pub mod group;`.
     - Tested: Accessed `scene.groups()`, created a group.
  5. **Implement Generic Batch Updates** (Pending, moved to Task 3):
     - Add `visible: bool` to `RenderObject` (default `true`) as a testable field.
     - Implement `GroupEditor::set_field<T>(&mut self, field: &str, value: T) -> Result<(), FieldError>`:
       - Supports all fields (e.g., `is_draggable`, `visible`, `offset`, `depth`) unless excluded.
       - Uses generics for type safety, with pattern matching on field names.
       - Define `FieldError { InvalidField, TypeMismatch, InvalidObject }`.
     - Test: Set `is_draggable`, `offset`, verify updates and type mismatch errors.
  6. **Optional Renderer Adjustment** (Deferred):
     - Modify `render_engine.rs` to skip rendering `!visible` objects.
     - Test: Hide a group via `set_field("visible", false)`, confirm objects disappear.
- **Constraints**: 
  - Groups have no rendering impact; `RenderObject`s remain independent.
  - Objects can belong to multiple groups; no field exclusions unless justified.
- **Notes**: 
  - Steps 1-4 complete; Step 5 moved to Task 3 for batch updates post-event bus.
  - Step 6 optional and deferred until visibility needed.

## Task 3: Complete Group Batch Updates
- **Goal**: Add generic batch update functionality to `GroupEditor` for efficient group operations using the event bus.
- **Affected Modules**: 
  - `src/gui_framework/scene/group.rs`
  - `src/gui_framework/scene/scene.rs`
- **Status**: Not started
- **Steps**:
  1. **Add Visibility Field**: Add `visible: bool` to `RenderObject` (default `true`).
  2. **Implement Batch Updates**: Add `GroupEditor::set_field<T>(&mut self, field: &str, value: T) -> Result<(), FieldError>`:
     - Supports fields like `is_draggable`, `visible`, `offset`, `depth`.
     - Emits `Event::FieldUpdated(object_id, field, value)` for each object.
     - Define `FieldError { InvalidField, TypeMismatch, InvalidObject }`.
  3. **Handle Events**: Update `Scene` to process `FieldUpdated` events, applying changes to `RenderObject`s.
  4. **Test**: In `main.rs`, create a group, set `visible` to `false`, verify event-driven updates.
- **Constraints**: Uses event bus; no rendering impact.
- **Notes**: Completes Task 2’s remaining goal; renderer adjustment deferred.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file to map keys to events, gracefully handling undefined hotkeys.
- **Affected Modules**:
  - `src/gui_framework/interaction/controller.rs`
  - New `src/gui_framework/input/hotkeys.rs`
  - `src/gui_framework/mod.rs`
- **Status**: Not started
- **Steps**:
  1. **Create `hotkeys.rs`**:
     - Define `struct HotkeyConfig { hotkeys: HashMap<String, Option<String>> }` (e.g., "Ctrl+/" -> Some("print_hello"), "Ctrl+Space" -> None).
     - Implement `load_config(path: &str) -> Result<Self, HotkeyError>` to parse TOML (using `toml` crate).
     - Define `HotkeyError { ParseError, FileNotFound }`.
  2. **Update `InteractionController`**:
     - Add `hotkeys: HotkeyConfig` to struct.
     - Extend `handle_event` to emit `Event::HotkeyPressed(key)` for keyboard events from `winit`.
  3. **Handle Hotkeys**: Subscriber (e.g., `main.rs`) processes `HotkeyPressed`, logs action or "No action defined" for `None`.
  4. **Integrate with `mod.rs`**: Export `hotkeys.rs`.
  5. **Test**:
     - Create `hotkeys.toml` with `Ctrl+/ = "print_hello"`, `Ctrl+. = "print_world"`, `Ctrl+Space = ""`.
     - Load in `main.rs`, press hotkeys, verify CLI output via events.
- **Constraints**: Use `toml` crate; null/empty mappings emit events but are no-ops unless subscribed.
- **Notes**: Ctrl+Space reserved for Task 7; leverages event bus.

## Task 5: Add Button Functionality
- **Goal**: Extend `gui_framework` to support button behavior on any `RenderObject` via events.
- **Affected Modules**:
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/interaction/controller.rs`
- **Status**: Not started
- **Steps**:
  1. **Update `RenderObject`**: Add `is_button: bool` and `action_id: Option<usize>` (ID for action, e.g., 1 = "Save").
  2. **Handle Button Clicks**: Extend `controller.rs` `handle_event` to emit `Event::ButtonClicked(action_id)` for `is_button` objects via `pick_object_at`.
  3. **Test**: Add a button-enabled object in `main.rs`, click it, verify `main.rs` logs "Button X clicked" via event subscription.
- **Constraints**: Event-driven; rendering unchanged.
- **Notes**: `action_id` triggers events; subscribers define behavior.

## Task 6: Add Text Display with Rendering Module
- **Goal**: Support static text display (e.g., field weights) with an extensible rendering module.
- **Affected Modules**:
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/rendering/renderable.rs`
  - New `src/gui_framework/rendering/text_renderer.rs`
  - New shaders (e.g., `text.vert`, `text.frag`)
  - `src/gui_framework/rendering/mod.rs`
- **Status**: Not started
- **Steps**:
  1. **Update `RenderObject`**: Add `text: Option<String>` and `font_size: f32`.
  2. **Create `text_renderer.rs`**:
     - Define `trait TextRenderable { render_text(&self, &mut VulkanContext, &RenderObject, &mut Vec<vk::CommandBuffer>); }`.
     - Implement `struct TextRenderer` with placeholder rendering (e.g., colored rectangles per character).
  3. **Update `buffer_manager.rs`**: Call `TextRenderer::render_text` for text-enabled objects.
  4. **Test**: Add a text object in `main.rs`, verify placeholder display.
- **Constraints**: Placeholder rendering initially; extensible for future features.
- **Notes**: Prepares for Task 7 text editing.

## Task 7: Implement Text Editing
- **Goal**: Add support for editable text fields using events.
- **Affected Modules**:
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/interaction/controller.rs`
  - `src/gui_framework/rendering/renderable.rs`
  - `src/gui_framework/rendering/text_renderer.rs`
- **Status**: Not started
- **Steps**:
  1. **Track Active Field**: Add `active_text_id: Option<usize>` to `Scene`.
  2. **Handle Keyboard Input**: Extend `controller.rs` to emit `Event::TextEdited(id, text)` for keyboard events on active fields.
  3. **Update `Scene`**: Subscribe to `TextEdited`, update `RenderObject` text.
  4. **Render Cursor**: Add `cursor_pos: usize` to `RenderObject`, update `text_renderer.rs` to render a blinking line.
  5. **Test**: Click a text field in `main.rs`, type characters, verify updates via events.
- **Constraints**: Basic ASCII input; defer advanced rendering.
- **Notes**: Event-driven; builds on Task 6.

## Task 8: Add Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by Ctrl+Space, with dynamic options on a semi-transparent torus.
- **Affected Modules**:
  - New `src/gui_framework/rendering/pie_menu.rs`
  - `src/gui_framework/interaction/controller.rs`
  - `src/gui_framework/scene/scene.rs`
  - `src/gui_framework/rendering/text_renderer.rs`
  - New shaders (`pie_menu.vert`, `pie_menu.frag`)
  - `src/gui_framework/rendering/mod.rs`
- **Status**: Not started
- **Steps**:
  1. **Create `pie_menu.rs`**:
     - Define `PieMenu { options: Vec<PieOption>, center: [f32; 2], radius: f32, inner_radius: f32, opacity: f32 }` and `PieOption { label: String, action_id: usize }`.
     - Implement `new(center: [f32; 2], options: Vec<PieOption>) -> Self` with torus geometry.
     - Render torus with custom shader, position text via `text_renderer.rs` evenly along radius.
  2. **Update `Scene`**: Add `active_menu: Option<PieMenu>`.
  3. **Handle Trigger**: In `controller.rs`, emit `Event::ShowPieMenu(position)` on Ctrl+Space; `Scene` subscribes to set `active_menu`.
  4. **Handle Selection**: Emit `Event::MenuOptionSelected(action_id)` on click; subscriber processes action.
  5. **Render Menu**: Extend `buffer_manager.rs` to call `PieMenu::render` if `active_menu` exists.
  6. **Test**: Press Ctrl+Space, verify torus menu with text options; select an option, confirm event.
- **Constraints**: Event-driven; torus with no middle, semi-transparent.
- **Notes**: Prepares for GUI builder; uses Task 6 text rendering.

## Task 9 (Pending): Initial `gui_app/dividers.rs` Outline
- **Goal**: Sketch divider system using enhanced API.
- **Affected Modules**: New `src/gui_app/dividers.rs`
- **Status**: Not started
- **Steps**:
  1. Define `Divider` struct with properties (e.g., position, size, orientation).
  2. Implement `DividerSystem` to manage dividers using `Scene` API (e.g., `add_object`).
  3. Test in `main.rs`: Add a divider, verify rendering and interaction.
- **Constraints**: Initial sketch; defer full implementation until event bus and GUI features are ready.
- **Notes**: Builds on event-driven framework.

## Dependencies
- Relies on `Scene` for state, `Renderer` for rendering, `EventBus` (Task 1) for communication.

## Future Considerations
- Build interactive GUI layout system post-Task 9.
- Enhance `text_renderer.rs` for markdown, syntax highlighting, code folding.