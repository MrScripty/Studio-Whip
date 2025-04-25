// Keep existing modules needed for Vulkan backend
pub mod rendering;
pub mod context;

// Add new ECS-related modules
pub mod components;
pub mod events;
pub mod plugins;

// Keep interaction module *only* for hotkeys for now
pub mod interaction;

// Keep exports needed by main.rs for Vulkan setup/rendering bridge
pub use context::vulkan_context::VulkanContext;
pub use context::vulkan_setup::{setup_vulkan, cleanup_vulkan};
// pub use rendering::render_engine::Renderer; // Keep Renderer export if needed by main.rs bridge

// Keep HotkeyConfig export
pub use interaction::hotkeys::{HotkeyConfig, HotkeyError};

// Remove old exports
// pub use scene::scene::{Scene, RenderObject, InstanceData};
// pub use interaction::controller::InteractionController;
// pub use event_bus::{EventBus, BusEvent, EventHandler};

// Export new components/events if needed directly (or use full path)
// pub use components::*;
// pub use events::*;