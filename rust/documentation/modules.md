# Modules Documentation for `rusty_whip` (March 19, 2025)

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It is currently migrating to the Bevy engine.**
### Current State (Post Task 6.1)
- **Bevy Integration**: Application runs using `bevy_app::App` and `bevy_winit`. Windowing is handled by Bevy. Core Bevy plugins (`Input`, `Log`, etc.) are active.
- **Bridging**: Legacy framework components (`VulkanContext`, `Scene`, `Renderer`, `EventBus`, `InteractionController`, `ClickRouter`) are wrapped in `Arc<Mutex<>>` and managed as Bevy `Resources`.
- **Lifecycle**: Bevy systems in `main.rs` orchestrate setup, updates, and cleanup, calling legacy functions where appropriate.
- **Rendering/Input**: Uses *placeholder* logic. `Renderer::new`/`render`/`resize` calls are stubbed due to `&mut VulkanContext` access issues with the resource bridge. Legacy `InteractionController` is inactive as input is not yet bridged from Bevy Input resources.
- **Features Implemented (Legacy)**: Event bus, logical grouping, visibility flag, configurable hotkey loading, generic click routing (via legacy bus), object pooling.
- **Features Active**: Bevy app structure, windowing, logging, core Vulkan setup via system, exit handling.
- **Features Inactive/Placeholder**: Rendering, resizing logic, input handling (click/drag/hotkey actions), event bus publishing from input.
- **Modules Removed**: `gui_framework/window/`.
- Task 1-5 **Complete** (Legacy). Task 6.1 **Complete**.

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
                event_bus.rs
                interaction/
                    controller.rs
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
                mod.rs
            lib.rs
            main.rs
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
- **Notes**: Public exports include legacy framework types.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, Bevy plugins (`WinitPlugin`, `InputPlugin`, `LogPlugin`, etc.), Bevy resources (`Arc<Mutex<>>` wrappers for legacy framework components), and Bevy systems (`Startup`, `Update`, `Last`) for lifecycle management. Systems handle Vulkan setup, placeholder renderer creation/calls, placeholder resize handling, exit requests, and cleanup triggers.
- **Key Structs**:
    - `VulkanContextResource(Arc<Mutex<VulkanContext>>)`
    - `SceneResource(Arc<Mutex<Scene>>)`
    - `EventBusResource(Arc<EventBus>)`
    - `RendererResource(Arc<Mutex<PlaceholderRenderer>>)` // Placeholder type used
    - `InteractionControllerResource(Arc<Mutex<InteractionController>>)`
    - `ClickRouterResource(Arc<Mutex<ClickRouter>>)`
    - `PlaceholderRenderer` // Temporary struct definition
    - `ClickRouter` // Legacy struct definition
    - `SceneEventHandler` // Legacy struct definition
    - `HotkeyActionHandler` // Legacy struct definition
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_vulkan_system(...) -> Result<(), String>`
    - `create_renderer_system(...) -> ()`
    - `handle_resize_system(...) -> ()`
    - `winit_event_bridge_system(...) -> ()` (Inactive placeholder)
    - `render_trigger_system(...) -> ()` (Placeholder logic)
    - `handle_close_request(...) -> ()`
    - `cleanup_system(...) -> ()`
- **Notes**: Uses Bevy App structure. Manages legacy state via `Resource` bridge pattern. Contains placeholder logic for rendering/resizing due to `&mut VulkanContext` access issues. Input handling deferred. Cleanup logic triggered by `AppExit` event.

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports submodules and key types (e.g., `Renderer`, `Scene`, `EventBus`, `InteractionController`).
- **Notes**: Removed `window` module export.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources (instance, device, allocator, etc.) and related state.
- **Key Structs**: `VulkanContext { entry, instance, surface_loader, surface, device, queue, queue_family_index: Option<u32>, allocator, swapchain_loader, swapchain, images, image_views, vertex_buffer, vertex_allocation, render_pass, framebuffers, vertex_shader, fragment_shader, pipeline_layout, pipeline, command_pool, command_buffers, image_available_semaphore, render_finished_semaphore, fence, current_image }`. **(Removed `window` field)**.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource` in `main.rs`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**:
  - `setup_vulkan(vk_ctx: &mut VulkanContext, window: &winit::window::Window) -> ()` **(Signature updated)**
  - `cleanup_vulkan(vk_ctx: &mut VulkanContext) -> ()`
- **Notes**: Called by `setup_vulkan_system` and `cleanup_system` in `main.rs`.

### `src/gui_framework/event_bus.rs`
- **Purpose**: **Legacy component.** Decouples components via a publish-subscribe event system.
- **Key Structs**: `EventBus { subscribers: Arc<Mutex<Vec<Arc<Mutex<dyn EventHandler>>>>> }`.
- **Key Enums**: `BusEvent { ObjectMoved(usize, [f32; 2], Option<usize>), InstanceAdded(usize, usize, [f32; 2]), ObjectPicked(usize, Option<usize>), ObjectClicked(usize, Option<usize>), RedrawRequested, HotkeyPressed(Option<String>) }`.
- **Key Traits**: `EventHandler: Send + Sync { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }`.
- **Key Methods**: `new() -> Self`, `subscribe_handler<H: EventHandler + 'static>(&self, H)`, `subscribe_arc<T: EventHandler + Send + Sync + 'static>(&self, Arc<Mutex<T>>)`, `publish(&self, BusEvent)`, `clear(&self)`.
- **Notes**: Managed via `EventBusResource`. Currently not receiving input-driven events. `clear()` called by `cleanup_system`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction controller and hotkey components.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: **Legacy component.** Handles mouse and keyboard input, loads hotkey configuration, tracks modifier state, performs hit-testing via `Scene`, and publishes interaction events (`ObjectMoved`, `ObjectClicked`, `HotkeyPressed`) via the legacy `EventBus`.
- **Key Structs**: `MouseState { is_dragging, last_position, dragged_object }`, `CursorContext { Canvas, Other }`, `InteractionController { mouse_state, context, hotkey_config, current_modifiers }`.
- **Key Methods**:
  - `new() -> Self` - Loads hotkey config from file relative to executable.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&Scene>, _renderer: Option<&mut Renderer>, window: &Window, event_bus: &Arc<EventBus>) -> ()` - Processes `winit` events.
- **Notes**: Managed via `InteractionControllerResource`. `handle_event` is currently **not called** by any active Bevy system. Input handling needs migration to Bevy Input. Depends on `hotkeys.rs` and `scene.rs`.

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML, error handling, and key event formatting logic.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }`, `HotkeyError { FileNotFound(String), ReadError(std::io::Error), ParseError(toml::de::Error) }`.
- **Key Methods**:
  - `HotkeyConfig::load_config(path: &Path) -> Result<Self, HotkeyError>` - Loads config from specified path.
  - `HotkeyConfig::get_action(&self, key_combo: &str) -> Option<&String>` - Looks up action for a formatted key string.
  - `format_key_event(modifiers: ModifiersState, key: PhysicalKey) -> Option<String>` - Converts winit key/modifier state to string (e.g., "Ctrl+S").
- **Notes**: Used by legacy `InteractionController`. Reads `hotkeys.toml` (expected to be copied to target dir by `build.rs`).

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `Renderable`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for drawing `Renderable` objects, handling instancing and visibility.
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], pipeline_layout: vk::PipelineLayout, _projection_descriptor_set: vk::DescriptorSet, extent: vk::Extent2D) -> ()`.
- **Notes**: Checks `renderable.visible`. Called by `render_engine.rs` and `resize_handler.rs` (part of placeholder logic).

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines properties of a Vulkan-renderable object, including instance buffer state and visibility.
- **Key Structs**: `Renderable { ..., depth: f32, ..., instance_count: u32, instance_buffer_capacity: u32, visible: bool }`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates Vulkan rendering, manages sub-component lifecycles (`PipelineManager`, `BufferManager`), handles legacy `InstanceAdded` events, and manages swapchain presentation.
- **Key Structs**: `Renderer { pipeline_manager: PipelineManager, buffer_manager: BufferManager, pending_instance_updates: Arc<Mutex<Vec<(usize, usize, [f32; 2])>>> }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self`
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) -> ()`
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene) -> ()`
  - `cleanup(self, platform: &mut VulkanContext) -> ()`
- **Event Handling**: `impl EventHandler for Renderer { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }`.
- **Notes**: Managed via `RendererResource` (using `PlaceholderRenderer` type currently). `new`, `render`, `resize_renderer` are called via placeholder logic in `main.rs` systems due to `&mut VulkanContext` issues. `cleanup` called by `cleanup_system`. Relies heavily on delegation. Cleanup order is critical.

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
- **Purpose**: Handles window resizing logic, updating swapchain, framebuffers, projection matrix, and renderable vertices.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(&mut VulkanContext, &mut Scene, &mut Vec<Renderable>, vk::PipelineLayout, vk::DescriptorSet, u32, u32, &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer` (part of placeholder logic).

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**: `create_swapchain(...) -> vk::SurfaceFormatKHR`, `create_framebuffers(...) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize` (part of placeholder logic).

### `src/gui_framework/scene/mod.rs`
- **Purpose**: Re-exports scene management modules.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages application state: renderable objects (`ElementPool` containing `RenderObject` with `visible` flag), logical groups (`GroupManager`), window dimensions. Publishes `InstanceAdded` events to legacy `EventBus`. Object state updated via legacy `SceneEventHandler`. Provides hit-testing capability.
- **Key Structs**: `RenderObject { vertices, vertex_shader_filename, fragment_shader_filename, depth, on_window_resize_scale, on_window_resize_move, offset, is_draggable, instances, visible }`, `InstanceData { offset: [f32; 2] }`, `ElementPool { elements, free_indices }`, `Scene { pool, groups, width, height, event_bus: Arc<EventBus> }`.
- **Key Traits**: `HitTestable { contains(...) -> bool }`.
- **Key Methods**: `new(...) -> Self`, `groups(...) -> &mut GroupManager`, `add_object(...) -> usize`, `add_instance(...) -> usize`, `pick_object_at(...) -> Option<(usize, Option<usize>)>`, `translate_object(...) -> ()`, `update_dimensions(...) -> ()`.
- **Notes**: Managed via `SceneResource`. `pick_object_at` used by legacy `InteractionController` (inactive). `translate_object` called by legacy `SceneEventHandler`. `update_dimensions` called by `handle_resize_system` (placeholder). `visible` flag added to `RenderObject`.

### `src/gui_framework/scene/group.rs`
- **Purpose**: Manages named, logical groups of object pool indices for organization.
- **Key Structs**: `Group { name, object_ids }`, `GroupManager { groups }`, `GroupEditor<'a> { group }`, `GroupError { DuplicateName, GroupNotFound }`.
- **Key Methods**: `GroupManager::new() -> Self`, `add_group(...)`, `delete_group(...)`, `group(...)`, `get_groups_with_object(...)`, `GroupEditor::add_object(...)`, `remove_object(...)`, `list_objects(...)`.
- **Notes**: Batch operations via events implemented in Task 3.

### `src/gui_framework/window/`
- **Notes**: Module removed. Windowing handled by `bevy_window`/`bevy_winit` via `main.rs`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders to SPIR-V using `glslc` and copies user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory (`target/<profile>/`).
- **Key Methods**: `main() -> ()`, `compile_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable. Uses environment variables (`CARGO_MANIFEST_DIR`, `PROFILE`) to determine paths.

## Shaders
- **Location**: `shaders/` (Source), copied to target build dir by `build.rs`.
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1), and instance offset (vertex attribute binding 1).

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `glam = "0.30"` (pending removal), `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**. (Removed `winit`).