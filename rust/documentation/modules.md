# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. Current focus: 2D GUI with Vulkan rendering, click-and-drag, instancing, logical grouping, visibility toggling, a configurable hotkey system, generic click handling, all driven by an event bus.
### Current State
- 2D GUI: Depth-sorted objects, orthographic projection, resizing, event-driven dragging/instancing, logical grouping, visibility checks during rendering.
- Features Implemented: Event bus (`ObjectClicked` added), mouse picking/dragging via events, generic click handling via `ClickRouter` pattern (in `main.rs`), instancing via events (fixed capacity buffer), group management (logical-only), Vulkan resource management refactored (`PipelineManager`, `BufferManager`), visibility flag (`RenderObject.visible`, `Renderable.visible`), configurable hotkey system (`hotkeys.toml`, `InteractionController`, `EventLoopProxy`), build script copies user config.
- Features Skipped: Instance buffer resizing, resize conflict handling, context switching, undo; batch group operations pending.
- Task 1 (Event Bus), Task 2 (Grouping), Task 3 (Group Batch Trigger), Task 3.1 (Visibility), Task 4 (Hotkeys), Task 5 (Generic Click Handling) are **Complete**.
- Application structure uses `EventLoop::run` with state managed in `main.rs` closure; `VulkanContextHandler` removed.

## Module Structure
Studio_Whip/
    LICENSE
    README.md
    .gitignore
    Rust/
        src/
            gui_framework/
                context/
                    vulkan_context.rs
                    vulkan_setup.rs
                    mod.rs
                event_bus.rs        # Updated
                interaction/
                    controller.rs   # Updated
                    hotkeys.rs
                    mod.rs
                rendering/
                    buffer_manager.rs
                    command_buffers.rs
                    pipeline_manager.rs
                    render_engine.rs
                    renderable.rs
                    resize_handler.rs
                    shader_utils.rs
                    swapchain.rs
                    mod.rs
                scene/
                    group.rs
                    scene.rs
                    mod.rs
                # window/ directory removed
                mod.rs
            lib.rs
            main.rs             # Updated
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
- **Purpose**: Defines the vertex type and re-exports `gui_framework` contents.
- **Key Structs**: `Vertex { position: [f32; 2] }`.
- **Notes**: Public exports include `Renderer`, `Scene`, `EventBus`, `InteractionController`, etc.

### `src/main.rs`
- **Purpose**: Entry point; sets up `winit` `EventLoop` with user events, manages core application state (`VulkanContext`, `Scene`, `Renderer`, `InteractionController`, `Window`) via `Option` fields within the `run` closure, handles the main event loop, creates `EventLoopProxy`, initializes `EventBus`, defines and manages application-level event handlers like `ClickRouter`, subscribes handlers (`HotkeyActionHandler`, `SceneEventHandler`, `Renderer`, `ClickRouter`), populates initial `Scene`, registers test click callbacks, and orchestrates cleanup on exit.
- **Key Structs**:
    - `UserEvent { Exit }`
    - `HotkeyActionHandler { proxy }`
    - `SceneEventHandler { scene_ref }`
    - `ClickRouter { callbacks: HashMap<usize, Box<Mutex<dyn FnMut(usize, Option<usize>) + Send + 'static>>> }`
- **Key Methods**:
    - `main() -> ()`
    - `impl EventHandler for HotkeyActionHandler { handle(&mut self, event: &BusEvent) }`
    - `impl EventHandler for SceneEventHandler { handle(&mut self, event: &BusEvent) }`
    - `impl EventHandler for ClickRouter { handle(&mut self, event: &BusEvent) }`
    - `ClickRouter::new() -> Self`
    - `ClickRouter::register_click_handler(&mut self, object_id: usize, callback: impl FnMut(...) + Send + 'static)`
- **Notes**: Uses `EventLoop::run` closure model. State managed locally via `Option` and `Arc<Mutex<>>`. `ClickRouter` provides application-specific click handling. Cleanup logic resides in `Event::LoopExiting`. Replaces `VulkanContextHandler`.

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports submodules and key types (e.g., `Renderer`, `Scene`, `EventBus`, `InteractionController`).

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources and related state.
- **Key Structs**: `VulkanContext { window, entry, instance, surface_loader, surface, device, queue, queue_family_index: Option<u32>, allocator, swapchain_loader, swapchain, images, image_views, render_pass, framebuffers, command_pool, command_buffers, image_available_semaphore, render_finished_semaphore, fence, current_image }`.
- **Key Methods**: `new() -> Self`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**:
  - `setup_vulkan(vk_ctx: &mut VulkanContext, window: Arc<Window>) -> ()`
  - `cleanup_vulkan(vk_ctx: &mut VulkanContext) -> ()`
- **Notes**: Used by `main.rs` closure during `Event::Resumed` and `Event::LoopExiting`.

### `src/gui_framework/event_bus.rs`
- **Purpose**: Decouples components via a publish-subscribe event system.
- **Key Structs**: `EventBus { subscribers: Arc<Mutex<Vec<Arc<Mutex<dyn EventHandler>>>>> }`.
- **Key Enums**: `BusEvent { ObjectMoved(usize, [f32; 2], Option<usize>), InstanceAdded(usize, usize, [f32; 2]), ObjectPicked(usize, Option<usize>), ObjectClicked(usize, Option<usize>), RedrawRequested, HotkeyPressed(Option<String>) }`.
- **Key Traits**: `EventHandler: Send + Sync { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }`.
- **Key Methods**: `new() -> Self`, `subscribe_handler<H: EventHandler + 'static>(&self, H)`, `subscribe_arc<T: EventHandler + Send + Sync + 'static>(&self, Arc<Mutex<T>>)`, `publish(&self, BusEvent)`, `clear(&self)`.
- **Notes**: `ObjectClicked` event added for generic click actions. `clear()` called during `Event::LoopExiting`. `subscribe_arc` requires `Send + Sync` on `T`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction controller and hotkey components.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse and keyboard input, loads hotkey configuration, tracks modifier state, performs hit-testing via `Scene`, and publishes interaction events (`ObjectMoved`, `ObjectClicked`, `HotkeyPressed`) via the `EventBus`.
- **Key Structs**: `MouseState { is_dragging, last_position, dragged_object }`, `CursorContext { Canvas, Other }`, `InteractionController { mouse_state, context, hotkey_config, current_modifiers }`.
- **Key Methods**:
  - `new() -> Self` - Loads hotkey config from file relative to executable.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&Scene>, _renderer: Option<&mut Renderer>, window: &Window, event_bus: &Arc<EventBus>) -> ()` - Processes `winit` events, updates `current_modifiers`, uses `format_key_event` and `hotkey_config`, calls `scene.pick_object_at` on mouse press, publishes `ObjectClicked` on hit, publishes `ObjectMoved` during drag, publishes `HotkeyPressed`.
- **Notes**: Depends on `hotkeys.rs` and `scene.rs`. Publishes `ObjectClicked` instead of `ObjectPicked` on mouse press hit detection. State includes `current_modifiers`.

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML, error handling, and key event formatting logic.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }`, `HotkeyError { FileNotFound(String), ReadError(std::io::Error), ParseError(toml::de::Error) }`.
- **Key Methods**:
  - `HotkeyConfig::load_config(path: &Path) -> Result<Self, HotkeyError>` - Loads config from specified path.
  - `HotkeyConfig::get_action(&self, key_combo: &str) -> Option<&String>` - Looks up action for a formatted key string.
  - `format_key_event(modifiers: ModifiersState, key: PhysicalKey) -> Option<String>` - Converts winit key/modifier state to string (e.g., "Ctrl+S").
- **Notes**: Used by `InteractionController`. Reads `hotkeys.toml` (expected to be copied to target dir by `build.rs`).

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `Renderable`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for drawing `Renderable` objects, handling instancing and visibility.
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], pipeline_layout: vk::PipelineLayout, _projection_descriptor_set: vk::DescriptorSet, extent: vk::Extent2D) -> ()`.
- **Notes**: Checks `renderable.visible` before issuing draw calls. Called by `render_engine.rs` and `resize_handler.rs`.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines properties of a Vulkan-renderable object, including instance buffer state and visibility.
- **Key Structs**: `Renderable { ..., depth: f32, ..., instance_count: u32, instance_buffer_capacity: u32, visible: bool }`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates Vulkan rendering, manages sub-component lifecycles (`PipelineManager`, `BufferManager`), handles `InstanceAdded` events, and manages swapchain presentation.
- **Key Structs**: `Renderer { pipeline_manager: PipelineManager, buffer_manager: BufferManager, pending_instance_updates: Arc<Mutex<Vec<(usize, usize, [f32; 2])>>> }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self`
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) -> ()`
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene) -> ()`
  - `cleanup(self, platform: &mut VulkanContext) -> ()`
- **Event Handling**: `impl EventHandler for Renderer { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }` - Handles `InstanceAdded`.
- **Notes**: Relies heavily on delegation. Cleanup order is critical. Called from `main.rs` closure.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
- **Key Structs**: `PipelineManager { pipeline_layout, descriptor_set_layout, descriptor_pool, descriptor_set }`.
- **Key Methods**: `new(&mut VulkanContext, &Scene) -> Self`, `cleanup(self, device: &ash::Device) -> ()`.
- **Notes**: Provides resources to `BufferManager`.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages Vulkan buffers, allocations, pipelines, shaders, descriptor sets, and populates `Renderable` state including visibility. Handles instance buffer creation/updates.
- **Key Structs**: `BufferManager { uniform_buffer, uniform_allocation, renderables: Vec<Renderable> }`.
- **Key Methods**:
  - `new(&mut VulkanContext, &Scene, vk::PipelineLayout, vk::DescriptorSetLayout, vk::DescriptorPool) -> Self` - Creates resources, copies `visible` flag from `RenderObject`.
  - `update_offset(...) -> ()`, `update_instance_offset(...) -> ()`, `update_instance_buffer(...) -> ()`.
  - `cleanup(mut self, device: &ash::Device, allocator: &vk_mem::Allocator, descriptor_pool: vk::DescriptorPool) -> ()`.
- **Notes**: Takes layout/pool from `PipelineManager`.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing, updating swapchain, framebuffers, projection matrix, and renderable vertices.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(&mut VulkanContext, &mut Scene, &mut Vec<Renderable>, vk::PipelineLayout, vk::DescriptorSet, u32, u32, &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**: `create_swapchain(...) -> vk::SurfaceFormatKHR`, `create_framebuffers(...) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`.

### `src/gui_framework/scene/mod.rs`
- **Purpose**: Re-exports scene management modules.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages application state: renderable objects (`ElementPool` containing `RenderObject` with `visible` flag), logical groups (`GroupManager`), window dimensions. Publishes `InstanceAdded` events. Object state updated via `SceneEventHandler`. Provides hit-testing capability.
- **Key Structs**: `RenderObject { vertices, vertex_shader_filename, fragment_shader_filename, depth, on_window_resize_scale, on_window_resize_move, offset, is_draggable, instances, visible }`, `InstanceData { offset: [f32; 2] }`, `ElementPool { elements, free_indices }`, `Scene { pool, groups, width, height, event_bus: Arc<EventBus> }`.
- **Key Traits**: `HitTestable { contains(...) -> bool }`.
- **Key Methods**: `new(...) -> Self`, `groups(...) -> &mut GroupManager`, `add_object(...) -> usize`, `add_instance(...) -> usize`, `pick_object_at(...) -> Option<(usize, Option<usize>)>`, `translate_object(...) -> ()`, `update_dimensions(...) -> ()`.
- **Notes**: `visible` flag added to `RenderObject`. `pick_object_at` used by `InteractionController`. State managed via `Arc<Mutex<>>` in `main.rs`.

### `src/gui_framework/scene/group.rs`
- **Purpose**: Manages named, logical groups of object pool indices for organization.
- **Key Structs**: `Group { name, object_ids }`, `GroupManager { groups }`, `GroupEditor<'a> { group }`, `GroupError { DuplicateName, GroupNotFound }`.
- **Key Methods**: `GroupManager::new() -> Self`, `add_group(...)`, `delete_group(...)`, `group(...)`, `get_groups_with_object(...)`, `GroupEditor::add_object(...)`, `remove_object(...)`, `list_objects(...)`.
- **Notes**: Batch operations via events implemented in Task 3.

### `src/gui_framework/window/`
- **Notes**: This module and `window_handler.rs` were removed. Event loop and window management logic now reside in `main.rs`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders to SPIR-V using `glslc` and copies user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory (`target/<profile>/`).
- **Key Methods**: `main() -> ()`, `compile_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable. Uses environment variables (`CARGO_MANIFEST_DIR`, `PROFILE`) to determine paths.

## Shaders
- **Location**: `shaders/` (Source), copied to target build dir by `build.rs`.
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1), and instance offset (vertex attribute binding 1).

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`