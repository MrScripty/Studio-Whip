# Plan for Implementing Changes in `rusty_whip` (Updated March 11, 2025)

## Project Overview: `rusty_whip`

### Purpose
`rusty_whip` is an advanced 3D & 2D content generation application designed for integration into a professional digital entertainment production pipeline. It prioritizes AI diffusion and inference performance on GPU VRAM, offering robust tools for prompt engineering, story writing, and multimedia creation. The app focuses on 2D video, stills, animation sequencing, audio editing, and story-driven workflows, with 3D primarily supporting image2image AI guidance. It features a client-side, quantum-resistant, cryptographically secure P2P networking system for live multi-user editing, project sharing, and peer discovery. The desktop app targets Linux and Windows, with unofficial cross-platform compiling for Mac and BSD.

### Key Features (Relevant to This Plan)
- **2D GUI**: Primary interface with depth-sorted render objects (e.g., background, widgets) in pixel coordinates, using orthographic projection for intuitive layout and resizing.
- **Background Sizing**: Must cover the entire window, resizing dynamically.
- **GUI Behaviors**: Most elements move to maintain relative positions (e.g., button stays near top-left); scaling deferred with a `RenderBehavior` hook.
- **Future 3D Viewports**: Separate 3D rendering with depth buffer, composited under 2D GUI using perspective projection.
- **Export**: Supports video sequences, EXR, OBJ, JPG, PNG files.

### Current State
- See modules.md

### Goals for This Plan
1. Add `f32` depth sorting for 2D render objects (background, triangle, square).
2. Implement orthographic projection with pixel coordinates for intuitive GUI layout using `glam`.
3. Enable window resizing, ensuring background fills the window and GUI elements move proportionately.
4. Set colors: background `21292a`, triangle `ff9800`, square `42c922`.

---

## Step-by-Step Plan

### Setup Details
- **Window Size**: 600x300 pixels, set explicitly in `main.rs`.
- **Colors**: 
  - Background: `21292a` (RGB: 33, 41, 42 → 0.129, 0.161, 0.165).
  - Triangle: `ff9800` (RGB: 255, 152, 0 → 1.0, 0.596, 0.0).
  - Square: `42c922` (RGB: 66, 201, 34 → 0.259, 0.788, 0.133).
- **Error Handling**: Use `unwrap` with `println!` for obvious failures (e.g., shader loading, GPU absence); robust handling deferred.
- **Shader Compilation**: Handled by `build.rs` with `glslc`; separate `compile_shaders.sh` and `.ps1` scripts available for user edits. Ensure all GLSL shaders are version 460.
- **Matrix Library**: Use `glam` (v0.30) for SIMD-optimized matrix operations.

### Step 1: Add `depth` Field and Background
- **Objective**: Extend `RenderObject` with `depth: f32` and `RenderBehavior`, add a background quad, keep NDC rendering.
- **Files**:
  - `Cargo.toml`: Add `glam = "0.30"`.
  - `scene.rs`: Add `depth: f32` and `behavior: RenderBehavior` to `RenderObject`, define:
    ```rust
    pub enum RenderBehavior {
        Move,        // Maintains relative position (default)
        Scale(f32),  // Scales size proportionally (deferred)
    }
    ```
  - `main.rs`: Set window to 600x300 via `Window::default_attributes().with_inner_size(winit::dpi::PhysicalSize::new(600, 300))`, add:
    - Background quad (`-1,-1` to `1,1`, `depth: 0.0`, `Move`).
    - Triangle (`depth: 1.0`, `Move`).
    - Square (`depth: 2.0`, `Move`).
  - Shaders: `background.frag`, `triangle.frag`, `square.frag` set colors (`21292a`, `ff9800`, `42c922`), reuse `triangle.vert` as `background.vert`.
- **Details**:
  - Background uses NDC quad for now.
- **Test**: `cargo run`—600x300 window, background, triangle, square render in vector order (no sorting yet).

### Step 2: Depth Sorting
- **Objective**: Sort `renderables` by `depth` in `Renderer` for consistent 2D layering.
- **Files**:
  - `renderer.rs`: Sort `renderables` by `depth` in `new` before pipeline creation:
    ```rust
    renderables.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());
    ```
- **Details**:
  - `RenderBehavior` unused in sorting (all `Move` for now).
- **Test**: `cargo run`—background (`0.0`) behind triangle (`1.0`) behind square (`2.0`), swap depths in `main.rs` to verify.

### Step 3: Uniform Buffer Setup
- **Objective**: Add uniform buffer with `glam` orthographic projection, update shaders, keep NDC coords temporarily.
- **Files**:
  - `renderer.rs`: Add fields (`uniform_buffer`, `uniform_allocation`, `descriptor_set_layout`, `descriptor_set`, `descriptor_pool`), create buffer with:
    ```rust
    use glam::Mat4;
    let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0).to_cols_array();
    ```
    - Set up descriptors (binding 0).
  - Shaders: Update `.vert` files:
    ```glsl
    layout(binding = 0) uniform UniformBufferObject {
        mat4 projection;
    } ubo;
    gl_Position = ubo.projection * vec4(inPosition, 0.0, 1.0);
    ```
- **Details**:
  - Matrix maps NDC directly for now (600x300 hardcoded).
- **Test**: `cargo run`—same rendering, uniform setup confirmed.

### Step 4: Switch to Screen-Space Coords
- **Objective**: Use pixel coords with orthographic projection, adjust GUI positions.
- **Files**:
  - `main.rs`: Update to pixel coords:
    - Background: `0,0` to `600,300`.
    - Triangle: Centered (e.g., `275,125` to `325,175`).
    - Square: Top-left quadrant (e.g., `100,50` to `150,100`).
  - `renderer.rs`: Bind descriptor set in command buffers, set clear color to `[0.0, 0.0, 0.0, 0.0]`.
- **Details**:
  - Positions hardcoded for 600x300, will adjust in Step 6.
- **Test**: `cargo run`—objects in pixel coords, depth sorted.

### Step 5: Extract Helper Functions
- **Objective**: Refactor `Renderer::new` for resizing prep, no functional change.
- **Files**:
  - `renderer.rs`: Extract:
    - `create_swapchain`.
    - `create_framebuffers`.
    - `record_command_buffers`.
- **Details**:
  - DRY focus, reuse existing logic.
- **Test**: `cargo run`—no rendering change.

### Step 6: Window Resizing with Proportionate Movement
- **Objective**: Enable resizing, background fills window, elements move proportionately.
- **Files**:
  - `window_management.rs`: Add `resizing: bool`, handle `Resized`:
    ```rust
    WindowEvent::Resized(size) => {
        self.resizing = true;
        self.renderer.as_mut().unwrap().resize(&mut self.platform, size.width, size.height);
        self.resizing = false;
    }
    ```
  - `renderer.rs`: Add `resize`:
    ```rust
    pub fn resize(&mut self, platform: &mut Platform, width: u32, height: u32) {
        let ortho = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0).to_cols_array();
        // Update uniform buffer, recreate swapchain, framebuffers, command buffers
        // TBD: Recalculate vertex positions based on width/height ratios (e.g., triangle center at `width/2, height/2`)
    }
    ```
    - Recalculation method TBD when implementing this step.
  - `main.rs`: Define initial positions as ratios (e.g., triangle center `width*0.5, height*0.5`).
- **Details**:
  - Background always `0,0` to `width,height`.
  - All objects use `RenderBehavior::Move`; scaling deferred.
- **Test**: `cargo run`, resize—background fills, elements move proportionately (e.g., triangle centered, square in top-left quadrant), depth sorted.

---

## Files Involved
1. **`Cargo.toml`**: Add `glam = "0.30"`.
2. **`build.rs`**: Unchanged.
3. **`main.rs`**: Window size, scene setup with depths and behaviors.
4. **`renderer.rs`**: Depth sorting, uniform buffer, resizing logic.
5. **`scene.rs`**: `depth` and `RenderBehavior` in `RenderObject`.
6. **`window_management.rs`**: Resizing event handling.
7. **`platform.rs`**: Unchanged.
8. **`vulkan_core.rs`**: Unchanged.
9. **`lib.rs`**: Exports updated `scene`.
10. **Shaders**: Updated `.vert` for uniforms, `.frag` for colors.

---

## Useful Details for Implementation

### Dependencies
- Updated: `ash 0.38`, `vk-mem 0.4`, `winit 0.30.9`, `raw-window-handle 0.6`, `ash-window 0.13`, `glam 0.30`.

### Window Size
- Set via `PhysicalSize::new(600, 300)` in `main.rs`.

### Shader Compilation
- `build.rs` compiles to `.spv`; scripts optional.

### Error Handling
- `unwrap` with `println!` for simplicity.

### GUI Behavior
- **Moving**: All elements (background, triangle, square) move proportionately.
- **Scaling**: Deferred via `RenderBehavior::Scale`.

### Future 3D
- 2D uses orthographic projection (`glam::Mat4::orthographic_rh`); 3D will use perspective (`glam::Mat4::perspective_rh`) with separate render passes.

### Testing
- Each step: `cargo build`, `cargo run`—verify colors, depth, resizing.

---

## Plan Execution
- **Start**: Step 1—adds `depth`, `RenderBehavior`, and background.
- **Progress**: Incremental, testable with `cargo run`.
- **Debug**: Revert on failure, check shaders/Vulkan errors with `println!`.