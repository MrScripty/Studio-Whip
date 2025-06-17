# Sample Parser Output

This is an example of what the Rust code parser would generate when analyzing the whip_ui codebase.

## Summary

- **Modules**: 45
- **Dependencies**: 234
- **Plugins**: 4

## Modules

### gui_framework::components

**Structs** (7): ShapeData, Visibility, Interaction, BackgroundQuad, TextData, TextLayoutOutput, TextRenderData

**Components** (5): ShapeData, Visibility, Interaction, TextData, TextLayoutOutput

### gui_framework::rendering

**Structs** (12): RenderEngine, BufferManager, TextRenderer, VulkanContext, PipelineManager, FontServer, GlyphAtlas, Swapchain, CommandBuffers, ResizeHandler, ShaderUtils

**Systems** (8): rendering_system, buffer_manager_despawn_cleanup_system, handle_resize_system, create_renderer_system, create_glyph_atlas_system, create_font_server_system, create_global_ubo_system, create_text_rendering_resources_system

### gui_framework::plugins

**Structs** (4): GuiFrameworkCorePlugin, GuiFrameworkInteractionPlugin, GuiFrameworkMovementPlugin, GuiFrameworkBindingsPlugin

## Bevy Plugins

### GuiFrameworkCorePlugin

**Systems**: setup_vulkan_system, create_renderer_system, create_glyph_atlas_system, text_layout_system, rendering_system, cleanup_trigger_system

**Resources**: VulkanContextResource, RendererResource, BufferManagerResource, GlyphAtlasResource

### GuiFrameworkInteractionPlugin

**Systems**: interaction_system, text_editing_system, text_drag_selection_system, load_hotkeys_system

**Resources**: HotkeyResource, MouseContext

### GuiFrameworkMovementPlugin

**Systems**: movement_system

### GuiFrameworkBindingsPlugin

**Systems**: bindings_system