# Tasks for `rusty_whip` GUI Framework Enhancements (March 19, 2025)

## Overview
These tasks enhance `gui_framework` to support a divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework will remain generic, with `gui_app` building the specific divider GUI atop it.

## Task 1: Add Element Pool and Batching to `scene.rs`
- **Goal**: Enable efficient creation and management of `RenderObject`s for dividers and regions (256+ elements).
- **Affected Modules**:
  - `src/gui_framework/scene/scene.rs`
- **Steps**:
  1. **Add `ElementPool`**:
     - Define `struct ElementPool { elements: Vec<RenderObject>, free_indices: Vec<usize> }`.
     - Implement `new(capacity: usize)`, `acquire(template: RenderObject) -> usize`, `release(index: usize)`.
     - Initial capacity: 10,000 elements.
  2. **Update `Scene`**:
     - Replace `render_objects: Vec<RenderObject>` with `pool: ElementPool`.
     - Modify `new()` to initialize `pool: ElementPool::new(10000)`.
  3. **Enhance Creation Methods**:
     - Update `add_object(&mut self, object: RenderObject) -> usize` to return pool index: `self.pool.acquire(object)`.
     - Add `add_objects(&mut self, templates: Vec<RenderObject>) -> Vec<usize>`: `templates.into_iter().map(|t| self.pool.acquire(t)).collect()`.
     - Add `update_element(&mut self, element_id: usize, new_offset: [f32; 2])`: Updates `pool.elements[element_id].offset`.
- **Constraints**:
  - Preserve unbatched `add_object` for unique objects (e.g., dividers).
  - Ensure batching supports 256+ elements efficiently.
- **Notes**: Batching is optional—use for regions, `add_object` for one-offs.

## Task 2: Implement Grouping in `scene.rs`
- **Goal**: Support hierarchical organization for dividers and regions.
- **Affected Modules**:
  - `src/gui_framework/scene/scene.rs`
- **Steps**:
  1. **Define `Group`**:
     - `struct Group { element_ids: Vec<usize>, is_draggable: bool }`.
  2. **Update `Scene`**:
     - Add `groups: Vec<Group>` field.
     - Add `add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize`:
       ```rust
       let ids = self.add_objects(elements);
       let group_id = self.groups.len();
       self.groups.push(Group { element_ids: ids, is_draggable });
       group_id
       ```
     - Add `add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>)`:
       ```rust
       let ids = self.add_objects(elements);
       self.groups[group_id].element_ids.extend(ids);
       ```
     - Update `translate_object(&mut self, group_id: usize, dx: f32, dy: f32)`:
       ```rust
       for &id in &self.groups[group_id].element_ids {
           self.pool.elements[id].offset[0] += dx;
           self.pool.elements[id].offset[1] += dy;
       }
       ```
     - Update `pick_object_at(&self, x: f32, y: f32) -> Option<usize>` to return draggable group IDs.
- **Constraints**:
  - Support single-element groups (dividers) and multi-element groups (regions).
- **Notes**: `gui_app` assigns dividers/regions to groups explicitly.

## Task 3: Add Instancing to Rendering (`render_engine.rs`, `command_buffers.rs`)
- **Goal**: Render 256+ elements per region efficiently with instancing.
- **Affected Modules**:
  - `src/gui_framework/rendering/renderable.rs`
  - `src/gui_framework/rendering/render_engine.rs`
  - `src/gui_framework/rendering/command_buffers.rs`
- **Steps**:
  1. **Define `InstanceData`**:
     - `struct InstanceData { offset: [f32; 2] }`.
  2. **Update `Renderable`**:
     - Add `instance_buffer: vk::Buffer`, `instance_allocation: vk_mem::Allocation`, `instance_count: u32`.
  3. **Enhance `Renderer`**:
     - Add `add_instanced_elements(&mut self, template: RenderObject, instances: Vec<InstanceData>) -> usize`:
       - Creates one `Renderable` with shared vertex buffer and instanced offsets.
       - Allocates `instance_buffer` with `vk_mem`.
     - Update `new` to process groups, using instancing for multi-element groups.
     - Update `update_offset` to handle instance buffers for groups.
  4. **Optimize `record_command_buffers`**:
     - Modify to reset buffers: `device.reset_command_buffer(buffer, vk::CommandBufferResetFlags::empty())`.
     - Use `vkCmdDrawInstanced` for instanced `Renderable`s:
       ```rust
       device.cmd_draw_instanced(command_buffer, vertex_count, renderable.instance_count, 0, 0);
       ```
- **Constraints**:
  - Support non-instanced rendering for single-element groups (e.g., dividers).
- **Notes**: Instancing reduces draw calls, critical for 256+ elements.

## Task 4: Implement Flexible Event System in `controller.rs`
- **Goal**: Allow `gui_app` to attach custom input handlers for dividers and elements.
- **Affected Modules**:
  - `src/gui_framework/interaction/controller.rs`
  - `src/gui_framework/window/window_handler.rs`
- **Steps**:
  1. **Define `EventHandler`**:
     - `trait EventHandler { fn handle(&mut self, event: &Event<()>, scene: &mut Scene, window: &Window); }`.
  2. **Update `InteractionController`**:
     - Add `handlers: HashMap<usize, Box<dyn EventHandler>>`.
     - Add `attach_handler(&mut self, id: usize, handler: impl EventHandler + 'static)`:
       ```rust
       self.handlers.insert(id, Box::new(handler));
       ```
     - Rewrite `handle_event`:
       ```rust
       fn handle_event(&mut self, event: &Event<()>, scene: &mut Scene, window: &Window) {
           if let Event::WindowEvent { event: win_event, .. } = event {
               match win_event {
                   WindowEvent::MouseInput { state, button, .. } => {
                       if let Some(id) = scene.pick_object_at(...) {
                           self.handlers.get_mut(&id).map(|h| h.handle(event, scene, window));
                       }
                   }
                   WindowEvent::KeyboardInput { input, .. } => {
                       // Dispatch to relevant handlers (e.g., Ctrl+click)
                   }
                   WindowEvent::CursorMoved { .. } => {
                       if let Some(id) = self.mouse_state.dragged_object {
                           self.handlers.get_mut(&id).map(|h| h.handle(event, scene, window));
                       }
                   }
               }
           }
       }
       ```
  3. **Update `window_handler.rs`**:
     - Pass `KeyboardInput` events to `controller.handle_event`.
- **Constraints**:
  - Support mouse and keyboard events (e.g., Ctrl+click for divider creation).
- **Notes**: Targeted system for now; plan for event bus transition.

## Task 5: Plan Event Bus Transition for P2P Collaboration
- **Goal**: Prepare `gui_framework` for future P2P multi-user input.
- **Affected Modules**:
  - New `src/event_bus.rs`
  - `src/gui_framework/interaction/controller.rs`
- **Steps**:
  1. **Define `EventBus`** (placeholder):
     - `enum AppEvent { MouseMoved { x: f32, y: f32, user_id: usize }, ... }`.
     - `struct EventBus { subscribers: Vec<Box<dyn Fn(&AppEvent)>> }`.
     - Methods: `subscribe`, `publish`.
  2. **Update `InteractionController`**:
     - Add optional `bus: Option<&EventBus>` in `new`.
     - Future: Subscribe to bus, translate events to handlers.
  3. **Integration Plan**:
     - Local input publishes to bus; P2P module syncs events across clients.
     - `gui_app` subscribes for divider updates.
- **Constraints**:
  - Implement later when P2P is prioritized; use targeted system now.
- **Notes**: Ensures forward compatibility without immediate complexity.

## Task 6: Update API Exports in `mod.rs`
- **Goal**: Expose new functionality to `gui_app`.
- **Affected Modules**:
  - `src/gui_framework/mod.rs`
- **Steps**:
  - Export `add_object`, `add_objects`, `add_group`, `add_to_group`, `update_element`, `add_instanced_elements`, `attach_handler`.
- **Notes**: Clean interface for divider system.

## Task 7: Initial `gui_app/dividers.rs` Outline
- **Goal**: Sketch divider system using enhanced API.
- **Affected Modules**:
  - New `src/gui_app/dividers.rs`
- **Steps**:
  1. **Define Structures**:
     - `struct Divider { id: usize, pos: f32, is_vertical: bool, start: f32, end: f32 }`.
     - `struct Region { id: usize, x: f32, y: f32, width: f32, height: f32 }`.
     - `struct DividerSystem { dividers: Vec<Divider>, regions: Vec<Region>, scene: Scene, controller: InteractionController }`.
  2. **Basic Methods**:
     - `add_divider(&mut self, pos: f32, is_vertical: bool)`: Uses `add_group` for a draggable divider.
     - `update_regions(&mut self)`: Computes regions from divider intersections, updates groups/instances.
- **Notes**: Full implementation follows framework updates.

## Dependencies
- Relies on `Scene` for pooling/grouping, `Renderer` for instancing, `InteractionController` for events.

## Future Considerations
- Transition to `EventBus` when P2P collaboration is implemented.
- Test performance with 256+ elements per region.