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

## Task 2 (In Progress): Implement Grouping in `scene.rs`
- **Goal**: Support hierarchical organization for dividers and regions, currently affects rendering (to be redesigned as logical containers).
- **Affected Modules**: `src/gui_framework/scene/scene.rs`
- **Status**: Partially complete; redesign planned but not implemented
- **Steps**:
  1. **Current `Group`** (Complete):
     - Defined `struct Group { element_ids: Vec<usize>, is_draggable: bool }`.
     - Added `add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize`: Creates group with pool indices.
     - Added `add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>)`: Extends group with new objects.
     - Updated `translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>)`: Handles group or object/instance translation.
     - Updated `pick_object_at(&self, x: f32, y: f32) -> Option<(usize, Option<usize>)>`: Prioritizes group IDs, then object/instance IDs.
  2. **Redesign `Group`** (Planned, Not Implemented):
     - Goal: Logical organization only, no rendering impact.
     - Proposed API: `group(&mut self) -> usize`, `add_to_group(&mut self, group_id: usize, object_id: usize)`.
     - `pick_object_at` to return pool indices only; group operations separate.
- **Constraints**: Supports single/multi-element groups; current implementation affects rendering (to be fixed).
- **Notes**: Redesign prompted by hitbox/dragging issues with grouped objects; `gui_app` uses groups for divider/region management.

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