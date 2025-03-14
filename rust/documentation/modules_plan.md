# Splitting Plan for `render_engine.rs`

This document outlines a step-by-step plan to split `render_engine.rs` into smaller, modular files within `src/gui_framework/rendering/`. The goal is to enhance modularity, readability, and maintainability while ensuring the program remains functional after each split. Each step extracts a specific component, updates dependencies, and allows testing with `cargo build` and `cargo run` to verify the application (a 600x300 window with background, triangle, and square) works correctly. The process reflects the state of `rusty_whip` as of March 13, 2025, post-naming audit and directory restructuring.

---

## Starting Point
- **File**: `src/gui_framework/rendering/render_engine.rs`
- **Structure**: Contains `load_shader`, `create_swapchain`, `create_framebuffers`, `record_command_buffers`, `Renderable` struct, `Renderer` struct, and `Renderer` methods (`new`, `resize_renderer`, `render`, `cleanup`).
- **Test Command**: After each step, run:
  - `cargo build`: Ensures compilation succeeds.
  - `cargo run`: Verifies the window renders with background (`21292a`), triangle (`ff9800`), and square (`42c922`), with resizing intact.

---

## Step 1: Split Off `shader_utils.rs`
### Purpose
Extract the `load_shader` function into a utility module for loading SPIR-V shaders, making it reusable across the rendering pipeline.

### Actions
1. **Create `shader_utils.rs`**:
   - **Path**: `src/gui_framework/rendering/shader_utils.rs`
   - **Contents**: Move the `load_shader` function from `render_engine.rs`.
   - **External Imports Needed**:
     - `ash::vk`: For `vk::ShaderModule`, `vk::ShaderModuleCreateInfo`, `vk::ShaderModuleCreateFlags`.
     - `std::fs`: For `fs::read`.
     - `std::marker::PhantomData`: For `_marker` field.
   - **Input Data**:
     - `device: &ash::Device`: Reference to the Vulkan device (from `VulkanContext.device`).
     - `filename: &str`: Shader file name (e.g., `"background.vert.spv"`) from `Scene.render_objects`.
   - **Output Data**:
     - Returns `vk::ShaderModule`: Shader module passed to `render_engine.rs` for `Renderable` creation.

2. **Update `rendering/mod.rs`**:
   - **Change**: Add `pub mod shader_utils;`.
   - **Purpose**: Declares the new submodule.

3. **Update `render_engine.rs`**:
   - **Change**: Remove the `load_shader` function definition.
   - **Add Import**: `use crate::gui_framework::rendering::shader_utils::load_shader;`.
   - **Data Flow**: Calls to `load_shader` in `Renderer::new` remain unchanged, now using the imported function.

4. **Test**:
   - Run `cargo build` and `cargo run`.
   - Verify the window renders correctly (shaders load and objects display as before).

---

## Step 2: Split Off `renderable.rs`
### Purpose
Extract the `Renderable` struct into its own file, isolating the data structure for renderable objects (e.g., background, triangle, square).

### Actions
1. **Create `renderable.rs`**:
   - **Path**: `src/gui_framework/rendering/renderable.rs`
   - **Contents**: Move the `Renderable` struct from `render_engine.rs`.
   - **External Imports Needed**:
     - `ash::vk`: For `vk::Buffer`, `vk::ShaderModule`, `vk::Pipeline`.
     - `vk_mem::Alloc`: For `vk_mem::Allocation`.
     - `crate::Vertex`: For `original_positions` (via `Vec<[f32; 2]>` matching `Vertex.position`).
   - **Input Data**: None (struct definition only).
   - **Output Data**: Defines `Renderable` type, used as `Vec<Renderable>` in `Renderer.vulkan_renderables`.

2. **Update `rendering/mod.rs`**:
   - **Change**: Add `pub mod renderable;`.
   - **Purpose**: Declares the new submodule.

3. **Update `render_engine.rs`**:
   - **Change**: Remove the `Renderable` struct definition.
   - **Add Import**: `use crate::gui_framework::rendering::renderable::Renderable;`.
   - **Data Flow**: Uses of `Renderable` in `Renderer` struct and methods remain unchanged, now referencing the imported type.

4. **Test**:
   - Run `cargo build` and `cargo run`.
   - Verify the window renders correctly (objects still draw with correct depth and resizing).

---

## Step 3: Split Off `swapchain.rs`
### Purpose
Extract `create_swapchain` and `create_framebuffers` into a module for managing Vulkan swapchain and framebuffer setup, handling presentation resources.

### Actions
1. **Create `swapchain.rs`**:
   - **Path**: `src/gui_framework/rendering/swapchain.rs`
   - **Contents**: Move `create_swapchain` and `create_framebuffers` from `render_engine.rs`.
   - **External Imports Needed**:
     - `ash::vk`: For `vk::Extent2D`, `vk::SurfaceFormatKHR`, `vk::SwapchainKHR`, `vk::Image`, `vk::ImageView`, `vk::RenderPass`, `vk::Framebuffer`, etc.
     - `ash::khr::swapchain`: For `swapchain::Device`.
     - `crate::gui_framework::context::vulkan_context::VulkanContext`: For `vulkan_context` parameter.
     - `std::marker::PhantomData`: For `_marker` fields.
   - **Input/Output Data for `create_swapchain`**:
     - **Input**: `vulkan_context: &mut VulkanContext`, `extent: vk::Extent2D`.
     - **Output**: Returns `vk::SurfaceFormatKHR`; modifies `vulkan_context.swapchain`, `images`, `image_views`, `swapchain_loader`.
   - **Input/Output Data for `create_framebuffers`**:
     - **Input**: `vulkan_context: &mut VulkanContext`, `extent: vk::Extent2D`, `surface_format: vk::SurfaceFormatKHR`.
     - **Output**: Modifies `vulkan_context.render_pass`, `vulkan_context.framebuffers`.

2. **Update `rendering/mod.rs`**:
   - **Change**: Add `pub mod swapchain;`.
   - **Purpose**: Declares the new submodule.

3. **Update `render_engine.rs`**:
   - **Change**: Remove `create_swapchain` and `create_framebuffers`.
   - **Add Import**: `use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};`.
   - **Data Flow**: Calls to `create_swapchain` and `create_framebuffers` in `Renderer::new` and `resize_renderer` remain unchanged, now using imported functions.

4. **Test**:
   - Run `cargo build` and `cargo run`.
   - Verify the window renders and resizes correctly (swapchain and framebuffers still function).

---

## Step 4: Split Off `command_buffers.rs`
### Purpose
Extract `record_command_buffers` into a module for managing Vulkan command buffer recording, isolating drawing commands.

### Actions
1. **Create `command_buffers.rs`**:
   - **Path**: `src/gui_framework/rendering/command_buffers.rs`
   - **Contents**: Move `record_command_buffers` from `render_engine.rs`.
   - **External Imports Needed**:
     - `ash::vk`: For `vk::PipelineLayout`, `vk::DescriptorSet`, `vk::Extent2D`, `vk::CommandBuffer`, `vk::CommandPool`, etc.
     - `crate::gui_framework::context::vulkan_context::VulkanContext`: For `vulkan_context` parameter.
     - `crate::gui_framework::rendering::renderable::Renderable`: For `renderables` parameter.
     - `std::marker::PhantomData`: For `_marker` fields.
   - **Input Data**: `vulkan_context: &mut VulkanContext`, `renderables: &[Renderable]`, `pipeline_layout: vk::PipelineLayout`, `descriptor_set: vk::DescriptorSet`, `extent: vk::Extent2D`.
   - **Output Data**: Modifies `vulkan_context.command_pool`, `vulkan_context.command_buffers`.

2. **Update `rendering/mod.rs`**:
   - **Change**: Add `pub mod command_buffers;`.
   - **Purpose**: Declares the new submodule.

3. **Update `render_engine.rs`**:
   - **Change**: Remove `record_command_buffers`.
   - **Add Import**: `use crate::gui_framework::rendering::command_buffers::record_command_buffers;`.
   - **Data Flow**: Calls to `record_command_buffers` in `Renderer::new` and `resize_renderer` remain unchanged, now using the imported function.

4. **Test**:
   - Run `cargo build` and `cargo run`.
   - Verify the window renders correctly (command buffers record and draw as before).

---

## Step 5: Finalize `render_engine.rs`
### Purpose
Reduce `render_engine.rs` to focus solely on `Renderer` orchestration, relying on the split utilities for specific tasks.

### Actions
1. **Update `render_engine.rs`**:
   - **Contents**: Retain only the `Renderer` struct and its methods (`new`, `resize_renderer`, `render`, `cleanup`).
   - **External Imports Needed**:
     - `ash::vk`: For `vk::PipelineLayout`, `vk::Semaphore`, etc.
     - `ash::khr::swapchain`: For `swapchain::Device`.
     - `vk_mem::Alloc`: For `vk_mem::Allocation`.
     - `crate::gui_framework::context::vulkan_context::VulkanContext`: For `vulkan_context`.
     - `crate::gui_framework::scene::scene::Scene`: For `scene`.
     - `crate::Vertex`: For vertex data.
     - `glam::Mat4`: For orthographic projection.
     - `std::marker::PhantomData`: For Vulkan structs.
     - `crate::gui_framework::rendering::shader_utils::load_shader`: For shader loading.
     - `crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers}`: For swapchain setup.
     - `crate::gui_framework::rendering::command_buffers::record_command_buffers`: For command recording.
     - `crate::gui_framework::rendering::renderable::Renderable`: For `vulkan_renderables`.
   - **Data Flow**:
     - `new`: Takes `vulkan_context`, `extent`, `scene`; calls split functions; returns `Renderer`.
     - `resize_renderer`: Takes `self`, `vulkan_context`, `width`, `height`; calls split functions.
     - `render`: Uses `vulkan_context.command_buffers` from `record_command_buffers`.
     - `cleanup`: Uses `vulkan_renderables` from `Renderable`.

2. **Test**:
   - Run `cargo build` and `cargo run`.
   - Verify the window renders and behaves as before (full functionality intact).

---

## Final Structure
After all steps:
src/gui_framework/rendering/
├── shader_utils.rs
├── swapchain.rs
├── command_buffers.rs
├── renderable.rs
├── render_engine.rs
└── mod.rs

### Final `rendering/mod.rs`
- **Contents**:
  - `pub mod shader_utils;`
  - `pub mod swapchain;`
  - `pub mod command_buffers;`
  - `pub mod renderable;`
  - `pub mod render_engine;`
  - `pub use render_engine::Renderer;`
  - `pub use renderable::Renderable;`

---

## Notes
- **Order**: Starts with `shader_utils.rs` and `renderable.rs` as they’re foundational, followed by `swapchain.rs` and `command_buffers.rs`, leaving `render_engine.rs` last for orchestration.
- **Testing**: Each step ensures the program runs by maintaining data flow through updated imports.
- **Dependencies**: Updated incrementally to keep communication intact (e.g., `Renderable` needed by `command_buffers.rs`).

## Next Steps
- Implement each step, testing after each split.
- Update `modules.md` to reflect the new files once complete.
