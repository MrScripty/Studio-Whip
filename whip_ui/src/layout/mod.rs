use bevy_ecs::prelude::*;
use std::sync::Mutex;
use taffy::{TaffyTree, Style, NodeId};

/// Core UI node component that marks an entity as part of the layout system
#[derive(Component, Debug)]
pub struct UiNode {
    /// Optional Taffy node ID for layout calculations
    pub taffy_node: Option<NodeId>,
    /// Whether this node needs layout recalculation
    pub needs_layout: bool,
}

impl Default for UiNode {
    fn default() -> Self {
        Self {
            taffy_node: None,
            needs_layout: true,
        }
    }
}

/// Component that wraps Taffy's Style for layout properties
#[derive(Component, Debug, Clone)]
pub struct Styleable(pub Style);

impl Default for Styleable {
    fn default() -> Self {
        Self(Style::default())
    }
}

/// Component that stores the Taffy node reference for an entity
#[derive(Component, Debug)]
pub struct TaffyNode(pub NodeId);

/// Resource that manages the Taffy layout tree
#[derive(Resource)]
pub struct TaffyResource(pub Mutex<TaffyTree<Entity>>);

impl Default for TaffyResource {
    fn default() -> Self {
        Self(Mutex::new(TaffyTree::new()))
    }
}

impl TaffyResource {
    /// Create a new Taffy resource
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the Taffy tree (locks the mutex)
    pub fn with_tree<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TaffyTree<Entity>) -> R,
    {
        let mut tree = self.0.lock().unwrap();
        f(&mut tree)
    }
}

/// Bundle for creating layout-enabled entities
#[derive(Bundle, Default)]
pub struct LayoutBundle {
    pub ui_node: UiNode,
    pub style: Styleable,
}

/// Bundle for entities that already have a Taffy node
#[derive(Bundle)]
pub struct TaffyBundle {
    pub ui_node: UiNode,
    pub style: Styleable,
    pub taffy_node: TaffyNode,
}

/// System set for layout-related systems
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayoutSet {
    /// Systems that create or modify Taffy nodes
    CreateNodes,
    /// Systems that calculate layout
    ComputeLayout,
    /// Systems that apply layout results to transforms
    ApplyLayout,
}