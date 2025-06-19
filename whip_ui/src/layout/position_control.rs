use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// Component that defines which system controls an entity's position
#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum PositionControl {
    /// Taffy layout system controls position (default for TOML widgets)
    Layout,
    /// Manual/interaction system controls position (for draggable items)
    Manual,
    /// Hybrid: Taffy sets initial position, then manual takes over when dragged
    LayoutThenManual,
}

impl Default for PositionControl {
    fn default() -> Self {
        PositionControl::Layout
    }
}

impl PositionControl {
    /// Check if this entity should be positioned by the layout system
    pub fn uses_layout(&self) -> bool {
        matches!(self, PositionControl::Layout | PositionControl::LayoutThenManual)
    }
    
    /// Check if this entity can be manually positioned/dragged
    pub fn allows_manual(&self) -> bool {
        matches!(self, PositionControl::Manual | PositionControl::LayoutThenManual)
    }
    
    /// Convert LayoutThenManual to Manual (called when dragging starts)
    pub fn take_manual_control(&mut self) {
        if matches!(self, PositionControl::LayoutThenManual) {
            *self = PositionControl::Manual;
        }
    }
    
    /// Check if the entity is in pure manual mode
    pub fn is_manual(&self) -> bool {
        matches!(self, PositionControl::Manual)
    }
}

/// Marker component to indicate an entity has been initially positioned by layout
/// Used to track LayoutThenManual entities that have received their initial position
#[derive(Component, Debug)]
pub struct LayoutPositioned;