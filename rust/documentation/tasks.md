# Tasks for `rusty_whip` GUI Framework Enhancements (March 27, 2025)

## Overview
These tasks enhance `gui_framework` to support a divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building the specific divider GUI atop it. Recent focus: fixing instancing and addressing group-related rendering issues.

## Task 1 (Complete): Add Element Pool and Batching to `scene.rs`
- **Goal**: Enable efficient creation and management of `RenderObject`s for dividers and regions (256+ elements).
- **Affected Modules**: `src/gui_framework/scene/scene.rs`
- **Status**: Complete
- **Steps**:
  1. **Added `ElementPool`**: `struct ElementPool { elements: Vec<RenderObject>, free_indices: Vec<usize> }` with `new`, `acquire`, `release`, `len`, `iter`, `iter_mut`, `get`.
  2. **Updated `Scene`**: Uses `pool: ElementPool`, initialized with 10,000 capacity.
  3. **Enhanced Creation**: `add_object` returns pool index, `add_objects` batches multiple objects, `update_element` modifies offsets.
- **Constraints**: Preserves unbatched `add_object` for unique objects; supports 256+ elements.
- **Notes**: Batching implemented via pool; no explicit render batching.

## Task 2 (In Progress): Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers for batch operations on `RenderObject`s, decoupled from rendering and interaction, supporting multiple group membership per object.
- **Affected Modules**: 
  - `src/gui_framework/scene/scene.rs`
  - New `src/gui_framework/scene/group.rs`
  - `src/gui_framework/mod.rs`
- **Status**: In progress; planning complete, implementation pending
- **Steps**:
  1. **Clean Up Old Group Logic**:
     - Remove `groups: Vec<Group>` field from `Scene` struct.
     - Remove methods `add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize` and `add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>)` from `scene.rs`.
     - Update `pick_object_at(&self, x: f32, y: f32) -> Option<(usize, Option<usize>)>` to return only pool indices, removing group ID prioritization.
     - Update `translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>)` to handle object/instance dragging only, removing group translation logic.
     - Update `main.rs` to remove old group creation (e.g., replace with individual object adds until new API is ready).
     - Test: Dragging works on individual objects/instances, no group interference.
  2. **Create `group.rs` with `Group` and `GroupManager`**:
     - Define `Group { name: String, object_ids: Vec<usize> }` for named groups with pool indices.
     - Define `GroupManager { groups: Vec<Group> }` with methods:
       - `new(&mut self, name: &str) -> Result<(), GroupError>`: Creates a unique named group.
       - `delete(&mut self, name: &str) -> Result<(), GroupError>`: Deletes a group.
       - `edit(&mut self, name: &str) -> Result<GroupEditor, GroupError>`: Returns editor for group modifications.
       - `groups_for_object(&self, object_id: usize) -> Vec<&str>`: Lists groups an object belongs to.
     - Define error enum `GroupError { DuplicateName, GroupNotFound }`.
     - Test: Create/delete groups, query empty membership.
  3. **Add `GroupEditor` to `group.rs`**:
     - Define `GroupEditor<'a> { group: &'a mut Group, scene: &'a mut Scene }` for editing a group with access to `Scene`.
     - Implement:
       - `add_object(&mut self, object_id: usize)`: Adds an object to the group.
       - `remove_object(&mut self, object_id: usize)`: Removes an object from the group.
     - Test: Add/remove objects, verify `object_ids` updates.
  4. **Integrate `GroupManager` into `Scene`**:
     - Add `groups: GroupManager` to `Scene` struct.
     - Update `Scene::new` to initialize `GroupManager`.
     - Add `Scene::groups(&mut self) -> &mut GroupManager` method.
     - Update `src/gui_framework/mod.rs` with `pub mod group;`.
     - Test: Access `scene.groups()`, create a group.
  5. **Implement Generic Batch Updates**:
     - Add `visible: bool` to `RenderObject` (default `true`) as a testable field.
     - Implement `GroupEditor::set_field<T>(&mut self, field: &str, value: T) -> Result<(), FieldError>`:
       - Supports all fields (e.g., `is_draggable`, `visible`, `offset`, `depth`) unless excluded.
       - Uses generics for type safety, with pattern matching on field names.
       - Define `FieldError { InvalidField, TypeMismatch, InvalidObject }`.
     - Test: Set `is_draggable`, `offset`, verify updates and type mismatch errors.
  6. **Optional Renderer Adjustment**:
     - Modify `render_engine.rs` to skip rendering `!visible` objects (deferrable).
     - Test: Hide a group via `set_field("visible", false)`, confirm objects disappear.
- **Constraints**: 
  - Groups have no rendering impact; `RenderObject`s remain independent.
  - Objects can belong to multiple groups; no field exclusions unless justified.
- **Notes**: 
  - Cleaning up old logic first ensures no conflicts with new design.
  - `group.rs` keeps logic modular, reducing `scene.rs` complexity.
  - Step 6 is optional and can be deferred until visibility is needed in `gui_app`.

## Task 3 (Complete): Add Instancing to Rendering
- **Goal**: Render 256+ elements per region efficiently with instancing.
- **Affected Modules**:
  - `src/gui_framework/rendering/renderable.rs`
  - `src/gui_framework/rendering/render_engine.rs`
  - `src/gui_framework/rendering/command_buffers.rs`
  - `src/gui_framework/scene/scene.rs`
- **Status**: Complete
- **Steps**:
  1. **Defined `InstanceData`**: `struct InstanceData { offset: [f32; 2] }`.
  2. **Updated `Renderable`**: Added `instance_buffer: Option<vk::Buffer>`, `instance_allocation: Option<vk_mem::Allocation>`, `instance_count: u32`.
  3. **Enhanced `Renderer`**:
     - `new` initializes `instance_buffer` for objects with instances, updates pipeline with instance binding.
     - `render` syncs instance offsets to `instance_buffer`.
     - Added `update_instance_offset(&mut self, device: &Device, allocator: &Allocator, object_index: usize, instance_id: usize, offset: [f32; 2])`.
  4. **Updated `command_buffers.rs`**: Uses `vkCmdDraw` with `instance_count + 1` for instanced objects, `instance_count=1` for non-instanced.
  5. **Added to `Scene`**: `add_instance(&mut self, object_id: usize, offset: [f32; 2]) -> usize` adds to `instances`.
- **Constraints**: Supports non-instanced rendering; fixed for instanced objects; non-instanced grouped objects problematic.
- **Notes**: Instancing resolves hitbox/dragging for instanced objects; Task 2 redesign addresses non-instanced group issues.

## Task 4 (Complete): Implement Flexible Event System in `controller.rs`
- **Goal**: Allow `gui_app` to handle input for dividers and elements.
- **Affected Modules**:
  - `src/gui_framework/interaction/controller.rs`
  - `src/gui_framework/window/window_handler.rs`
- **Status**: Complete (targeted system)
- **Steps**:
  1. **Defined `MouseState`**: `dragged_object: Option<(usize, Option<usize>)>` for object/instance IDs.
  2. **Updated `InteractionController`**:
     - `handle_event` processes mouse input, supports instance dragging, fixed dragging-without-hit bug.
  3. **Updated `window_handler.rs`**: Passes events to `controller`.
- **Constraints**: Supports mouse events; keyboard (e.g., Ctrl+click) not implemented.
- **Notes**: Targeted system sufficient; event bus planned for P2P.

## Task 5 (Pending): Plan Event Bus Transition for P2P Collaboration
- **Goal**: Prepare `gui_framework` for P2P multi-user input.
- **Affected Modules**: New `src/event_bus.rs`, `src/gui_framework/interaction/controller.rs`
- **Status**: Not started
- **Steps**: (Unchanged)
- **Constraints**: Implement later when P2P prioritized.
- **Notes**: No progress; future plan.

## Task 6 (Complete): Update API Exports in `mod.rs`
- **Goal**: Expose functionality to `gui_app`.
- **Affected Modules**: `src/gui_framework/mod.rs`
- **Status**: Complete
- **Steps**: Exports `add_object`, `add_instance`, `add_group`, `add_to_group`, `update_element`.
- **Notes**: Reflects current API; may adjust post-Task 2 redesign.

## Task 7 (Pending): Initial `gui_app/dividers.rs` Outline
- **Goal**: Sketch divider system using enhanced API.
- **Affected Modules**: New `src/gui_app/dividers.rs`
- **Status**: Not started
- **Steps**: (Unchanged)
- **Notes**: Awaiting Task 2 completion.

## Dependencies
- Relies on `Scene` for pooling/instancing, `Renderer` for rendering, `InteractionController` for events.

## Future Considerations
- Complete group redesign (Task 2).
- Transition to `EventBus` for P2P.
- Test performance with 256+ elements.