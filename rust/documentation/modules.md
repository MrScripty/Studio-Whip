# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It is currently migrating its core logic to the Bevy engine (v0.15), strictly avoiding Bevy's rendering stack.**
### Current State (Post Task 6.3 Step 7 Preparation)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, and other core non-rendering Bevy plugins.
- **ECS Migration**: Core application state (transforms, shapes, visibility, interaction flags) managed via Bevy ECS components. User input processed by Bevy systems, triggering Bevy events. Old `Scene`, `EventBus`, `InteractionController`, `ClickRouter` removed.
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource`.
- **Rendering Status**: Rendering is triggered by a Bevy system (`rendering_system`) which collects `RenderCommandData` from ECS. The custom Vulkan backend (`Renderer`, `BufferManager`, etc.) has had signatures/structs adapted, but the core logic to create/update Vulkan resources based on ECS data and perform draw calls **is still placeholder/pending implementation**. Visual output does not reflect the ECS state. `Renderable` struct removed.
- **Features Active**: Bevy app structure, windowing, logging, input handling (click, drag, hotkeys via `ButtonInput`), ECS component/event usage, `bevy_transform`, core Vulkan setup, hotkey loading.
- **Features Inactive/Placeholder**: Actual scene rendering based on ECS data, Vulkan buffer/descriptor management based on ECS data.
- **Modules Removed**: `gui_framework/scene/`, `gui_framework/event_bus.rs`, `gui_framework/interaction/controller.rs`, `gui_framework/rendering/renderable.rs`.
- Task 1-5 **Complete** (Legacy). Task 6.1, 6.2 **Complete**. Task 6.3 Steps 1-6, 8 **Complete**. Step 7 **Partially Complete (Preparation Phase)**.

## Module Structure
Studio_Whip/
    LICENSE
    README.md
    .gitignore
    Rust/
        src/
            gui_framework/
                components/             # NEW: Bevy ECS Components
                    interaction.rs
                    mod.rs
                    shape_data.rs
                    visibility.rs
                context/
                    vulkan_context.rs
                    vulkan_setup.rs
                    mod.rs
                events/                 # NEW: Bevy Events
                    interaction_events.rs
                    mod.rs
                interaction/            # Modified: Only hotkeys remain
                    hotkeys.rs
                    mod.rs
                rendering/              # Modified: Adapting to ECS
                    buffer_manager.rs   # Needs major rework
                    command_buffers.rs  # Needs rework
                    mod.rs
                    pipeline_manager.rs # Adapted
                    render_engine.rs    # Needs rework
                    # renderable.rs     # REMOVED
                    resize_handler.rs   # Adapted
                    shader_utils.rs
                    swapchain.rs
                mod.rs
            lib.rs
            main.rs                     # Heavily modified for Bevy App/ECS/Systems
        shaders/
            # ... shader files ...
        user/
            hotkeys.toml
        Documentation/
            # ... docs ...
        utilities/
            # ... utils ...
        Cargo.toml
        build.rs
        compile_shaders.ps1
        compile_shaders.sh


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines shared types (`Vertex`, `RenderCommandData`, `PreparedDrawData`).
- **Key Structs**:
    - `Vertex { position: [f32; 2] }`
    - `RenderCommandData { entity_id: Entity, transform_matrix: Mat4, vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String, depth: f32 }`
    - `PreparedDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, descriptor_set: vk::DescriptorSet }`
- **Notes**: Uses `ash::vk`. `Arc` used in `RenderCommandData`.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, core Bevy plugins (Input, Transform, Window, Log, etc., *excluding rendering*), Bevy resources (`VulkanContextResource`, `RendererResource`, `HotkeyResource`), custom Bevy events, and Bevy systems for lifecycle management, ECS setup, input processing, state updates, and triggering the custom Vulkan rendering backend.
- **Key Structs (Defined In `main.rs`)**:
    - `VulkanContextResource(Arc<Mutex<VulkanContext>>)`
    - `RendererResource(Arc<Mutex<Renderer>>)` // Uses actual Renderer
    - `HotkeyResource(HotkeyConfig)`
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_vulkan_system(...) -> Result<(), String>`: Initializes Vulkan context.
    - `create_renderer_system(...) -> Result<(), String>`: Creates `RendererResource`.
    - `setup_scene_ecs(...) -> ()`: Loads `HotkeyResource`, spawns initial ECS entities.
    - `interaction_system(...) -> ()`: Processes mouse clicks/drags, writes `EntityClicked`/`EntityDragged` events.
    - `hotkey_system(...) -> ()`: Processes keyboard input, writes `HotkeyActionTriggered` events.
    - `movement_system(...) -> ()`: Updates `Transform` components based on `EntityDragged` events.
    - `app_control_system(...) -> ()`: Handles application exit based on `HotkeyActionTriggered` events.
    - `handle_resize_system(...) -> ()`: Calls `Renderer::resize_renderer`.
    - `rendering_system(...) -> ()`: Queries ECS (`GlobalTransform`, `ShapeData`, `Visibility`), collects `RenderCommandData`, calls `Renderer::render`.
    - `handle_close_request(...) -> ()`: Sends `AppExit` on window close request.
    - `cleanup_system(...) -> ()`: Cleans up `RendererResource` and `VulkanContextResource` on `AppExit`.
- **Notes**: Uses Bevy App structure. Manages Vulkan state via `Resource` bridge pattern. Rendering logic uses actual `Renderer` but its internal implementation is placeholder. Input handled by Bevy systems.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules. Exports types needed for Vulkan backend and hotkey config.
- **Notes**: No longer declares/exports `scene`, `event_bus`, or `renderable`. `interaction` module only contains `hotkeys`.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity.
- **Key Structs**: `ShapeData { vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String }`.
- **Notes**: Uses `Arc` for vertices. Used by `rendering_system` to create `RenderCommandData`.

### `src/gui_framework/components/visibility.rs`
- **Purpose**: Defines a custom visibility component to avoid `bevy_render`.
- **Key Structs**: `Visibility(pub bool)`.
- **Notes**: Used by `rendering_system` to filter visible entities.

### `src/gui_framework/components/interaction.rs`
- **Purpose**: Defines interaction properties for an entity.
- **Key Structs**: `Interaction { clickable: bool, draggable: bool }`.
- **Notes**: Used by `interaction_system` to determine how entities react to input.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources (instance, device, allocator, etc.) and related state.
- **Key Structs**: `VulkanContext { ... }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource` in `main.rs`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**: `setup_vulkan(vk_context: &mut VulkanContext, window: &winit::window::Window) -> ()`, `cleanup_vulkan(vk_context: &mut VulkanContext) -> ()`.
- **Notes**: Called by Bevy systems in `main.rs`. Still uses `winit::window::Window`.

### `src/gui_framework/events/mod.rs`
- **Purpose**: Declares and re-exports Bevy Event modules.
- **Modules**: `interaction_events.rs`.

### `src/gui_framework/events/interaction_events.rs`
- **Purpose**: Defines Bevy events related to user interaction.
- **Key Structs**: `EntityClicked { entity: Entity }`, `EntityDragged { entity: Entity, delta: Vec2 }`, `HotkeyActionTriggered { action: String }`.
- **Notes**: Written by input systems (`interaction_system`, `hotkey_system`), read by logic systems (`movement_system`, `app_control_system`).

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Declares interaction-related submodules (currently only `hotkeys`).

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML and error handling.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }`, `HotkeyError { ... }`.
- **Key Methods**: `HotkeyConfig::load_config(...) -> Result<Self, HotkeyError>`, `HotkeyConfig::get_action(...) -> Option<&String>`.
- **Notes**: Loaded into `HotkeyResource` by `setup_scene_ecs` in `main.rs`. Used by `hotkey_system`.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers based on `PreparedDrawData`. **Currently uses placeholder draw logic.**
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, prepared_draws: &[PreparedDrawData], pipeline_layout: vk::PipelineLayout, extent: vk::Extent2D) -> ()`.
- **Notes**: Called by `render_engine.rs`. Consumes `PreparedDrawData` generated (placeholder) by `BufferManager`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering. Manages `PipelineManager`, `BufferManager`. Calls `BufferManager` to prepare resources and `command_buffers` to record draws. **Partially adapted for ECS data flow, but relies on placeholder logic in sub-modules.** Uses `bevy_math::Mat4`.
- **Key Structs**: `Renderer { pipeline_manager: PipelineManager, buffer_manager: BufferManager, current_extent: vk::Extent2D }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self`
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) -> ()`
  - `render(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> ()`
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()`
- **Notes**: Managed via `RendererResource`. Called by `rendering_system`. Cleanup uses `&mut self`. Needs `BufferManager` rework to function correctly.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout` (defining bindings 0=global, 1=per-object), and `DescriptorPool`.
- **Key Structs**: `PipelineManager { pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**: `new(platform: &mut VulkanContext) -> Self`, `cleanup(&mut self, device: &ash::Device) -> ()`.
- **Notes**: Provides layout/pool to `BufferManager` for per-entity set allocation. Cleanup uses `&mut self`.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: **Component undergoing major rework.** Manages the global projection uniform buffer and per-entity Vulkan resources (buffers, pipelines, shaders, descriptor sets) based on ECS `RenderCommandData`. **Currently contains placeholder logic.**
- **Key Structs**:
    - `EntityRenderResources { vertex_buffer, vertex_allocation, vertex_count, offset_uniform, offset_allocation, descriptor_set, pipeline, vertex_shader, fragment_shader }` (Placeholder)
    - `BufferManager { uniform_buffer, uniform_allocation, entity_cache: HashMap<Entity, EntityRenderResources>, descriptor_set_layout, descriptor_pool }`
- **Key Methods**:
  - `new(platform: &mut VulkanContext, pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Self`
  - `prepare_frame_resources(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> Vec<PreparedDrawData>` (**Placeholder Implementation**)
  - `cleanup(&mut self, platform: &mut VulkanContext, descriptor_pool: vk::DescriptorPool) -> ()`
- **Notes**: Takes layout/pool from `PipelineManager`. Needs implementation for creating/updating resources in `prepare_frame_resources` based on `RenderCommandData`. Cleanup uses `&mut self`.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic for Vulkan swapchain and projection uniform buffer update.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(vulkan_context: &mut VulkanContext, new_extent: vk::Extent2D, uniform_allocation: &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`. Vertex resizing logic removed. Includes error handling for buffer mapping.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**: `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`, `create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders to SPIR-V using `glslc` and copies user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory.
- **Key Methods**: `main() -> ()`, `compile_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable.

## Shaders
- **Location**: `shaders/` (Source), copied to target build dir by `build.rs`.
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1), and instance offset (vertex attribute binding 1). **Integration with ECS components via `BufferManager` pending.**

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**, **`bevy_transform = "0.15"`**. (Removed `glam`). `winit = "0.30.9"` (Used internally by `bevy_winit` and `vulkan_setup`).