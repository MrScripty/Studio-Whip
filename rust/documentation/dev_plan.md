# Revised Plan for Implementing Click-and-Drag in `rusty_whip` (March 17, 2025)

## Project Overview: `rusty_whip`

### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application for digital entertainment production, emphasizing GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows. It features a client-side, quantum-resistant, P2P networking system for real-time multi-user editing and targets Linux/Windows with unofficial Mac/BSD support. The 2D GUI uses Vulkan for depth-sorted rendering, evolving into a full production tool with 3D viewports.

### Key Features (Relevant to This Plan)
- **2D GUI**: Depth-sorted render objects (background `21292a`, triangle `ff9800`, square `42c922`) in pixel coordinates, orthographic projection, resizable 600x300 window.
- **Click-and-Drag**: Reposition objects dynamically with shader-based offsets, context-aware input, Ctrl+Z undo via double buffering.
- **Future 3D**: Separate 3D rendering under 2D GUI, leveraging shader positioning.

### Current State (Post-Step 1)
- Mouse events forwarded to `InteractionController` and logged (`controller.rs`, `window_handler.rs`).
- Rendering functional with depth sorting, orthographic projection, resizing (`render_engine.rs`, `scene.rs`).
- See provided files: `controller.rs`, `command_buffers.rs`, `render_engine.rs`, `renderable.rs`, `swapchain.rs`, `scene.rs`, `window_handler.rs`.

### Goals for This Plan
1. Enable click-and-drag for `RenderObject`s using shader offsets.
2. Add context-aware input via `InteractionController` (e.g., `Canvas` drags, `Other` doesn’t).
3. Implement Ctrl+Z undo with double buffering.

---

## Step-by-Step Plan

### Setup Details
- **Window Size**: 600x300 pixels (`main.rs` via `PhysicalSize`).
- **Colors**: Background `21292a` (0.129, 0.161, 0.165), triangle `ff9800` (1.0, 0.596, 0.0), square `42c922` (0.259, 0.788, 0.133).
- **Error Handling**: Use `unwrap` with `println!` for simplicity.
- **Shader Compilation**: `build.rs` with GLSL 460; scripts optional.
- **Test Command**: `cargo build` (compiles), `cargo run` (renders window, tests functionality).

### Step 1: Detect and Log Mouse Events (Complete)
- **Objective**: Forward mouse events to `InteractionController` and log press, move, release actions.
- **Files**:
  - `src/gui_framework/window/window_handler.rs`: Forward `MouseInput` and `CursorMoved` to `controller.handle_event`.
  - `src/gui_framework/interaction/controller.rs`: Log events with `println!`.
- **Outcome**: Console logs "Pressed at", "Moved to", "Released at" with positions.
- **Test**: `cargo run`—click/drag logs (e.g., "Pressed at [0.0, 0.0]", "Moved to [105.0, 55.0]", "Released at [105.0, 55.0]").

### Step 2: Track Mouse State
- **Objective**: Update `MouseState` to track dragging state and deltas, enforce `Canvas` context.
- **Files**:
  - `src/gui_framework/interaction/controller.rs`:
    - On press: Set `is_dragging = true`, log "Dragging started at".
    - On move: If `Canvas` and dragging, log "Dragging delta", update `last_position`.
    - On release: If `Canvas` and dragging, set `is_dragging = false`, log "Dragging stopped at".
- **Outcome**: Logs "Dragging started at", "Dragging delta", "Dragging stopped at" only in `Canvas` context; `last_position` tracks correctly.
- **Test**: `cargo run`—drag logs (e.g., "Dragging started at [0.0, 0.0]", "Dragging delta: [5.0, 5.0]", "Dragging stopped at Some([105.0, 55.0])"); toggle `context` to `Other` in `new()`, no logs.

### Step 3: Add Offset Infrastructure to Scene and Renderable
- **Objective**: Add `offset` to `RenderObject` and uniform buffers to `Renderable` without altering rendering.
- **Files**:
  - `src/gui_framework/scene/scene.rs`: Add `pub offset: [f32; 2]` to `RenderObject`.
  - `src/gui_framework/rendering/renderable.rs`: Add `pub offset_uniform: vk::Buffer`, `pub offset_allocation: vk_mem::Allocation`.
  - `src/gui_framework/rendering/render_engine.rs`:
    - In `new`: Create `offset_uniform` and `offset_allocation` per `Renderable`, initialize with `obj.offset`.
    - In `cleanup`: Free `offset_uniform` and `offset_allocation`.
  - `src/gui_framework/main.rs`: Set `offset: [0.0, 0.0]` for each `RenderObject`.
- **Outcome**: Objects render at original positions (no visual change), `offset` fields present and initialized, no errors.
- **Test**: `cargo run`—window renders background, triangle, square as before; Step 2 logs persist; no crashes.

### Step 4: Enable Shader-Based Positioning with Per-Object Descriptor Sets
- **Objective**: Update shaders and renderer to use per-object descriptor sets for projection and offset uniforms.
- **Files**:
  - Shaders (`background.vert`, `triangle.vert`, `square.vert`): Add `layout(binding = 1) uniform Offset { vec2 offset; };`, update `gl_Position = ubo.projection * vec4(inPosition + offset, 0.0, 1.0);`.
  - `src/gui_framework/rendering/renderable.rs`: Add `pub descriptor_set: vk::DescriptorSet`.
  - `src/gui_framework/rendering/render_engine.rs`:
    - In `new`: Allocate descriptor sets (1 for projection + 1 per `Renderable`), bind projection to binding 0 (shared), offset to binding 1 (per-object).
  - `src/gui_framework/rendering/command_buffers.rs`: Update to bind each `Renderable`’s `descriptor_set`.
- **Outcome**: Objects render with initial offsets ([0.0, 0.0]), no visual change from Step 3 unless offsets manually set; shader infrastructure ready.
- **Test**: `cargo run`—renders as before; manually set `offset: [50.0, 50.0]` for triangle in `main.rs`, confirm it shifts 50 pixels right/up; Step 2 logs persist.

### Step 5: Implement Object Picking and Dragging
- **Objective**: Detect clicked objects and update their `offset` during dragging with visual feedback.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Add `pub trait HitTestable { fn contains(&self, x: f32, y: f32) -> bool; }`.
    - Impl `HitTestable` for `RenderObject`: Use bounding box check.
    - Add `pub fn pick_object_at(&self, x: f32, y: f32) -> Option<usize>`: Return topmost object index.
    - Add `pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32)`: Update `offset`.
  - `src/gui_framework/interaction/controller.rs`:
    - On press: Use `pick_object_at` to set `dragged_object`.
    - On move: If dragging, call `translate_object` with delta, request redraw.
  - `src/gui_framework/window/window_handler.rs`: Pass `&mut scene` to `handle_event`.
- **Outcome**: Clicking and dragging triangle/square moves them visually; logs show object index and deltas.
- **Test**: `cargo run`—drag triangle/square, they move (e.g., "Clicked object: 1", "Dragging delta: [5.0, 5.0]"), positions update on screen.

### Step 6: Optimize Dragging and Handle Conflicts
- **Objective**: Prevent resize interference and test context switching.
- **Files**:
  - `src/gui_framework/interaction/controller.rs`: Add `if !vulkan_context_handler.resizing` check to drag logic; test `CursorContext::Other`.
  - `src/gui_framework/window/window_handler.rs`: Ensure `resizing` blocks dragging in `handle_event` calls.
- **Outcome**: Dragging pauses during resize; no dragging in `Other` context.
- **Test**: `cargo run`—resize window, dragging pauses; toggle `context` to `Other` in `new()`, no dragging logs or movement.

### Step 7: Add Undo with Double Buffering
- **Objective**: Implement Ctrl+Z undo using double buffering for offsets.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Add `pub pending_offset: [f32; 2]` to `RenderObject`.
    - Update `translate_object`: Modify `pending_offset`.
    - Add `pub fn commit_offset(&mut self, index: usize)`: Copy `pending_offset` to `offset`.
    - Add `pub fn revert_offset(&mut self, index: usize)`: Reset `pending_offset` to `offset`.
  - `src/gui_framework/interaction/controller.rs`:
    - On move: Use `pending_offset`.
    - On `KeyboardInput` (Ctrl+Z): Call `revert_offset`, request redraw.
    - On release: Call `commit_offset`.
  - `src/gui_framework/rendering/render_engine.rs`: Sync `pending_offset` to uniforms in `render`.
- **Outcome**: Dragging updates position; Ctrl+Z reverts to last committed position; release commits changes.
- **Test**: `cargo run`—drag triangle, press Ctrl+Z to undo, release to commit; visuals and logs (e.g., "Dragging delta: [5.0, 5.0]") match actions.

---

## Files Involved
1. `src/gui_framework/interaction/mod.rs`
2. `src/gui_framework/interaction/controller.rs`
3. `src/gui_framework/mod.rs`
4. `src/gui_framework/window/window_handler.rs`
5. `src/gui_framework/scene/scene.rs`
6. `src/gui_framework/rendering/renderable.rs`
7. `src/gui_framework/rendering/render_engine.rs`
8. `src/gui_framework/rendering/command_buffers.rs`
9. `src/main.rs`
10. `shaders/background.vert`, `shaders/triangle.vert`, `shaders/square.vert`

---

## Necessary Files and Information
- **Provided**: `controller.rs`, `command_buffers.rs`, `render_engine.rs`, `renderable.rs`, `swapchain.rs`, `scene.rs`, `window_handler.rs`.
- **Assumed Available**: `main.rs`, `vulkan_context.rs`, `shader_utils.rs`, `build.rs`, shaders (`background.vert`, `triangle.vert`, `square.vert`).
- **Needed if Different**:
  - Exact `main.rs` constructor for `VulkanContextHandler`.
  - Shader variations (if not GLSL 460 or binding 0 differs).

---

## Notes
- **Starting Point**: Step 1 complete—mouse events logged, no visual changes yet.
- **Bite-Sized Steps**: Each step is small, builds incrementally, and has a testable outcome.
- **Shader Dependency**: Step 4 assumes shader updates; test with manual offsets to confirm readiness.
- **Future Plans**:
  - **Event Bus**: Replace `InteractionController` with an event bus for P2P collaboration later.
  - **ECS**: Refactor `RenderObject` into components for 2D/3D scalability when needed.
- **Reasoning**:
  - Split Steps 3 and 4 ensure shader setup is testable before dragging logic.
  - Early visual feedback in Step 5 aids debugging.
  - Context and undo build on a stable dragging system.