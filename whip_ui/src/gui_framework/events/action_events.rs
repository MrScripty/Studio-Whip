use bevy_ecs::prelude::*;
use std::collections::HashMap;

/// Core action event that represents a user-triggered action
#[derive(Event, Debug, Clone)]
pub struct ActionEvent {
    /// The action identifier (e.g., "navigate_home", "toggle_settings")
    pub action: String,
    /// Entity that triggered the action
    pub source_entity: Entity,
    /// Optional parameters for the action
    pub params: Option<HashMap<String, toml::Value>>,
    /// The original UI event that triggered this action (e.g., "click", "hover")
    pub event_type: String,
}

impl ActionEvent {
    pub fn new(action: String, source_entity: Entity, event_type: String) -> Self {
        Self {
            action,
            source_entity,
            params: None,
            event_type,
        }
    }

    pub fn with_params(mut self, params: HashMap<String, toml::Value>) -> Self {
        self.params = Some(params);
        self
    }

    /// Get a parameter value as a specific type
    pub fn get_param<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.params
            .as_ref()?
            .get(key)
            .and_then(|value| T::deserialize(value.clone()).ok())
    }

    /// Get a string parameter
    pub fn get_string_param(&self, key: &str) -> Option<String> {
        self.params
            .as_ref()?
            .get(key)
            .and_then(|value| value.as_str().map(|s| s.to_string()))
    }

    /// Get a float parameter
    pub fn get_float_param(&self, key: &str) -> Option<f64> {
        self.params
            .as_ref()?
            .get(key)
            .and_then(|value| value.as_float())
    }

    /// Get an integer parameter
    pub fn get_integer_param(&self, key: &str) -> Option<i64> {
        self.params
            .as_ref()?
            .get(key)
            .and_then(|value| value.as_integer())
    }

    /// Get a boolean parameter
    pub fn get_bool_param(&self, key: &str) -> Option<bool> {
        self.params
            .as_ref()?
            .get(key)
            .and_then(|value| value.as_bool())
    }
}

/// Built-in action types that the framework handles automatically
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinAction {
    /// Debug action that logs information
    Debug { message: String },
    /// Navigation action (placeholder for future router integration)
    Navigate { target: String },
    /// Toggle visibility of an entity
    ToggleVisibility { target_id: String },
    /// Update text content of an entity
    UpdateText { target_id: String, new_text: String },
    /// Trigger focus on an entity
    SetFocus { target_id: String },
}

impl BuiltinAction {
    /// Parse an action string and parameters into a builtin action
    pub fn from_action_event(event: &ActionEvent) -> Option<Self> {
        match event.action.as_str() {
            "debug" => {
                let message = event.get_string_param("message")
                    .unwrap_or_else(|| format!("Debug action triggered by {:?}", event.source_entity));
                Some(BuiltinAction::Debug { message })
            }
            "navigate" => {
                let target = event.get_string_param("target")?;
                Some(BuiltinAction::Navigate { target })
            }
            "toggle_visibility" => {
                let target_id = event.get_string_param("target_id")?;
                Some(BuiltinAction::ToggleVisibility { target_id })
            }
            "update_text" => {
                let target_id = event.get_string_param("target_id")?;
                let new_text = event.get_string_param("text")?;
                Some(BuiltinAction::UpdateText { target_id, new_text })
            }
            "set_focus" => {
                let target_id = event.get_string_param("target_id")?;
                Some(BuiltinAction::SetFocus { target_id })
            }
            _ => None,
        }
    }
}

/// Resource that tracks registered action handlers
#[derive(Resource, Default)]
pub struct ActionRegistry {
    /// Registered custom action handlers
    pub handlers: HashMap<String, Box<dyn ActionHandler>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a custom action handler
    pub fn register_handler<H: ActionHandler + 'static>(&mut self, action_name: String, handler: H) {
        self.handlers.insert(action_name, Box::new(handler));
    }

    /// Check if an action is registered
    pub fn has_action(&self, action_name: &str) -> bool {
        self.handlers.contains_key(action_name) || BuiltinAction::is_builtin(action_name)
    }
}

/// Trait for custom action handlers
pub trait ActionHandler: Send + Sync {
    /// Execute the action with the given parameters
    fn execute(&self, event: &ActionEvent, world: &mut World);
}

impl BuiltinAction {
    /// Check if an action name corresponds to a builtin action
    pub fn is_builtin(action_name: &str) -> bool {
        matches!(action_name, "debug" | "navigate" | "toggle_visibility" | "update_text" | "set_focus")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    #[test]
    fn test_action_event_creation() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        
        let action = ActionEvent::new(
            "test_action".to_string(),
            entity,
            "click".to_string(),
        );
        
        assert_eq!(action.action, "test_action");
        assert_eq!(action.source_entity, entity);
        assert_eq!(action.event_type, "click");
        assert!(action.params.is_none());
    }

    #[test]
    fn test_action_event_with_params() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        
        let mut params = HashMap::new();
        params.insert("message".to_string(), toml::Value::String("Hello".to_string()));
        params.insert("count".to_string(), toml::Value::Integer(42));
        
        let action = ActionEvent::new(
            "debug".to_string(),
            entity,
            "click".to_string(),
        ).with_params(params);
        
        assert_eq!(action.get_string_param("message"), Some("Hello".to_string()));
        assert_eq!(action.get_integer_param("count"), Some(42));
        assert_eq!(action.get_string_param("nonexistent"), None);
    }

    #[test]
    fn test_builtin_action_parsing() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        
        let mut params = HashMap::new();
        params.insert("message".to_string(), toml::Value::String("Test message".to_string()));
        
        let action_event = ActionEvent::new(
            "debug".to_string(),
            entity,
            "click".to_string(),
        ).with_params(params);
        
        let builtin = BuiltinAction::from_action_event(&action_event).unwrap();
        assert_eq!(builtin, BuiltinAction::Debug { message: "Test message".to_string() });
    }

    #[test]
    fn test_builtin_action_detection() {
        assert!(BuiltinAction::is_builtin("debug"));
        assert!(BuiltinAction::is_builtin("navigate"));
        assert!(!BuiltinAction::is_builtin("custom_action"));
    }
}