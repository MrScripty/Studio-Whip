# Plan for Implementing Click-and-Drag in `rusty_whip` (March 16, 2025)

## Project Overview: `rusty_whip`

### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application for digital entertainment production, emphasizing GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows. It features a client-side, quantum-resistant, P2P networking system for real-time multi-user editing and targets Linux/Windows with unofficial Mac/BSD support. The 2D GUI uses Vulkan for depth-sorted rendering, evolving into a full production tool with 3D viewports.

### Key Features (Relevant to This Plan)
- **2D GUI**: Depth-sorted render objects (background `21292a`, triangle `ff9800`, square `42c922`) in pixel coordinates, orthographic projection, resizable 600x300 window.
- **Click-and-Drag**: Reposition objects dynamically with shader-based offsets, context-aware input, Ctrl+Z undo via double buffering.
- **Future 3D**: Separate 3D rendering under 2D GUI, leveraging shader positioning.

### Current State
- See `modules.md` (March 13, 2025): Depth sorting, orthographic projection, resizing implemented; `render_engine.rs` split into modules.

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
- **Test Command**: `cargo build` (compiles), `cargo run` (renders window, tests dragging/undo).

### Step 0: Setup Interaction Controller
- **Objective**: Create `InteractionController` with `CursorContext` for input handling.
- **Files**:
  - `src/gui_framework/interaction/mod.rs`:
    - `pub mod controller; pub use controller::InteractionController;`
  - `src/gui_framework/interaction/controller.rs`:
    - `pub struct MouseState { pub is_dragging: bool, pub last_position: Option<[f32; 2]>, pub dragged_object: Option<usize> }`
    - `pub enum CursorContext { Canvas, Other } // Context at cursor position`
    - `pub struct InteractionController { pub mouse_state: MouseState, pub context: CursorContext }`
    - `impl InteractionController { pub fn new() -> Self { Self { mouse_state: MouseState { is_dragging: false, last_position: None, dragged_object: None }, context: CursorContext::Canvas } } }`
  - `src/gui_framework/mod.rs`:
    - Add: `pub mod interaction; pub use interaction::controller::InteractionController;`
- **Test**: `cargo build`—verify compilation.

### Step 1: Detect and Log Mouse Events
- **Objective**: Forward mouse events to controller, log actions.
- **Files**:
  - `src/gui_framework/window/window_handler.rs`:
    - Struct `VulkanContextHandler`: Add `controller: InteractionController`.
    - Fn `new`: Add `controller: InteractionController::new()`.
    - Fn `window_event`: Add:
      - `WindowEvent::MouseInput { state, button, .. }`: `if button == MouseButton::Left { self.controller.handle_event(event, None, None, &self.window); }`
      - `WindowEvent::CursorMoved { .. }`: `self.controller.handle_event(event, None, None, &self.window);`
  - `src/gui_framework/interaction/controller.rs`:
    - Fn: `pub fn handle_event(&mut self, event: &winit::event::Event<()>, _scene: Option<&Scene>, _renderer: Option<&mut Renderer>, window: &winit::window::Window) { match event { WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => println!("Pressed at {:?}", self.mouse_state.last_position.unwrap_or([0.0, 0.0])), WindowEvent::CursorMoved { position, .. } => { let pos = [position.x as f32, position.y as f32]; println!("Moved to {:?}", pos); self.mouse_state.last_position = Some(pos); }, WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => println!("Released at {:?}", self.mouse_state.last_position.unwrap_or([0.0, 0.0])), _ => (), } }`
- **Test**: `cargo run`—click/drag/release logs positions.

### Step 2: Track Mouse State
- **Objective**: Update `MouseState` for dragging, context-aware.
- **Files**:
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event`:
      - `MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }`: `if self.context == CursorContext::Canvas { self.mouse_state.is_dragging = true; let pos = self.mouse_state.last_position.unwrap_or([0.0, 0.0]); println!("Dragging started at {:?}", pos); }`
      - `CursorMoved { position, .. }`: `let pos = [position.x as f32, position.y as f32]; if self.context == CursorContext::Canvas && self.mouse_state.is_dragging { let delta = [pos[0] - self.mouse_state.last_position.unwrap()[0], pos[1] - self.mouse_state.last_position.unwrap()[1]]; println!("Dragging delta: {:?}", delta); self.mouse_state.last_position = Some(pos); } else { self.mouse_state.last_position = Some(pos); }`
      - `MouseInput { state: ElementState::Released, button: MouseButton::Left, .. }`: `if self.context == CursorContext::Canvas && self.mouse_state.is_dragging { self.mouse_state.is_dragging = false; println!("Dragging stopped at {:?}", self.mouse_state.last_position); }`
- **Test**: `cargo run`—drag logs deltas in `Canvas` context only.

### Step 3: Shader Positioning Setup
- **Objective**: Add offset for shader-based dragging.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Struct `RenderObject`: Add `offset: [f32; 2] = [0.0, 0.0]`.
  - `src/gui_framework/rendering/renderable.rs`:
    - Struct `Renderable`: Add `offset_uniform: vk::Buffer`, `offset_allocation: vk_mem::Allocation`.
  - `src/gui_framework/rendering/render_engine.rs`:
    - Fn `new`: For each `Renderable`:
      - Create `offset_uniform`: `let buffer_info = vk::BufferCreateInfo { size: 8, usage: vk::BufferUsageFlags::UNIFORM_BUFFER, ..Default::default() }; let allocation_info = vk_mem::AllocationCreateInfo { usage: vk_mem::MemoryUsage::CpuToGpu, ..Default::default() }; let (buffer, allocation) = vulkan_context.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info).unwrap(); unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(&allocation).unwrap().copy_from_slice(&render_object.offset); vulkan_context.allocator.as_ref().unwrap().unmap_memory(&allocation); }`
      - Update `descriptor_set_layout`: Add binding `vk::DescriptorSetLayoutBinding { binding: 1, descriptor_type: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 1, stage_flags: vk::ShaderStageFlags::VERTEX, ..Default::default() }`.
      - Update `descriptor_set`: Bind `offset_uniform` (binding 1).
    - Fn `cleanup`: Free `offset_uniform`, `offset_allocation`.
  - `shaders/background.vert`, `shaders/triangle.vert`, `shaders/square.vert`:
    - Add: `layout(binding = 1) uniform Offset { vec2 offset; };`
    - Update: `gl_Position = ubo.projection * vec4(inPosition + offset, 0.0, 1.0);`
- **Test**: `cargo run`—objects render as before (offset 0).

### Step 4: Identify Clicked Object
- **Objective**: Add hit detection with bounding boxes.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Trait: `pub trait HitTestable { fn contains(&self, x: f32, y: f32) -> bool; }`
    - Impl `HitTestable` for `RenderObject`: `fn contains(&self, x: f32, y: f32) -> bool { let (min_x, max_x, min_y, max_y) = self.vertices.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |acc, v| (acc.0.min(v[0]), acc.1.max(v[0]), acc.2.min(v[1]), acc.3.max(v[1]))); x >= min_x && x <= max_x && y >= min_y && y <= max_y }`
    - Fn: `pub fn pick_object_at(&self, x: f32, y: f32) -> Option<usize> { self.objects.iter().enumerate().filter(|(_, obj)| obj.contains(x, y)).max_by(|a, b| a.1.depth.partial_cmp(&b.1.depth).unwrap_or(Ordering::Equal)).map(|(i, _)| i) }`
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event` (press): `if let Some(scene) = scene { let pos = self.mouse_state.last_position.unwrap(); if let Some(index) = scene.pick_object_at(pos[0], pos[1]) { self.mouse_state.dragged_object = Some(index); println!("Clicked object: {:?}", index); } }`
- **Test**: `cargo run`—click logs object index.

### Step 5: Update Object Offset
- **Objective**: Adjust `offset` on drag.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Fn: `pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32) { let obj = &mut self.objects[index]; obj.offset[0] += dx; obj.offset[1] += dy; }`
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event` (move): `if let Some(scene) = scene { if let Some(index) = self.mouse_state.dragged_object { let delta = [pos[0] - self.mouse_state.last_position.unwrap()[0], pos[1] - self.mouse_state.last_position.unwrap()[1]]; scene.translate_object(index, delta[0], delta[1]); window.request_redraw(); } }`
- **Test**: `cargo run`—drag moves object visually.

### Step 6: Sync Offset to Renderable
- **Objective**: Update shader uniform with new offset.
- **Files**:
  - `src/gui_framework/rendering/render_engine.rs`:
    - Fn: `pub fn update_offset(&mut self, index: usize, offset: [f32; 2]) { let renderable = &mut self.vulkan_renderables[index]; unsafe { self.vulkan_context.allocator.as_ref().unwrap().map_memory(&renderable.offset_allocation).unwrap().copy_from_slice(&offset); self.vulkan_context.allocator.as_ref().unwrap().unmap_memory(&renderable.offset_allocation); } }`
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event` (move): `if let Some(renderer) = renderer { renderer.update_offset(index, scene.objects[index].offset); }`
- **Test**: `cargo run`—drag is smooth, persists.

### Step 7: Optimize and Polish
- **Objective**: Handle resize conflicts, test context.
- **Files**:
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event` (drag cases): Add `if !vulkan_context_handler.resizing { ... }`.
    - Add test: `self.context = CursorContext::Other;` (manual toggle).
  - `src/gui_framework/window/window_handler.rs`:
    - Update `handle_event` call: `self.controller.handle_event(event, Some(&self.scene), Some(&mut self.renderer), &self.window);`
- **Test**: `cargo run`—drag paused during resize, no drag in `Other`.

### Step 8: Double Buffering with Ctrl+Z Undo
- **Objective**: Add undo via double buffering.
- **Files**:
  - `src/gui_framework/scene/scene.rs`:
    - Struct `RenderObject`: Add `pending_offset: [f32; 2] = [0.0, 0.0]`.
    - Update `translate_object`: Use `pending_offset`.
    - Fn: `pub fn commit_offset(&mut self, index: usize) { self.objects[index].offset = self.objects[index].pending_offset; }`
    - Fn: `pub fn revert_offset(&mut self, index: usize) { self.objects[index].pending_offset = self.objects[index].offset; }`
  - `src/gui_framework/interaction/controller.rs`:
    - Update `handle_event` (move): Sync `pending_offset`.
    - Add: `WindowEvent::KeyboardInput { input: KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Z), modifiers: ModifiersState { ctrl: true, .. }, .. }, .. } => if let Some(scene) = scene { if let Some(index) = self.mouse_state.dragged_object { scene.revert_offset(index); renderer.unwrap().update_offset(index, scene.objects[index].offset); window.request_redraw(); } }`
    - Update (release): `scene.unwrap().commit_offset(self.mouse_state.dragged_object.unwrap());`
- **Test**: `cargo run`—drag, Ctrl+Z undoes, release commits.

---

## Files Involved
1. `src/gui_framework/interaction/mod.rs`
2. `src/gui_framework/interaction/controller.rs`
3. `src/gui_framework/mod.rs`
4. `src/gui_framework/window/window_handler.rs`
5. `src/gui_framework/scene/scene.rs`
6. `src/gui_framework/rendering/renderable.rs`
7. `src/gui_framework/rendering/render_engine.rs`
8. `shaders/background.vert`, `shaders/triangle.vert`, `shaders/square.vert`

---

## Necessary Files and Information
- **Provided**: `vulkan_context.rs`, `render_engine.rs`, `triangle.vert`, `window_handler.rs`, `modules.md`, `Cargo.toml`, directory structure.
- **Assumed Available**: `main.rs`, `scene.rs`, `build.rs`, other shaders (`background.vert`, `square.vert`).
- **Needed if Different**: 
  - Exact `main.rs` constructor for `VulkanContextHandler`.
  - Shader variations (if not GLSL 460 or binding 0 differs).

---

## Notes
- **Future Plans**:
  - **Event Bus**: Replace `InteractionController` with an event bus for P2P collaboration (e.g., `ObjectMoved` events), when subsystems grow.
  - **ECS**: Refactor `RenderObject` into components (e.g., `Position`, `RenderData`) for 2D/3D scalability, when complexity increases.
- **Reasoning**:
  - **Controller**: Simplifies input now, extensible to event bus.
  - **Shader Positioning**: High performance, 3D-ready.
  - **Double Buffering**: Enables undo, added post-dragging for clarity.
  - **CursorContext**: Region-specific input, future-proof for GUI regions.

---

## Updated `modules_plan.md`