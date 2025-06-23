use bevy_ecs::prelude::*;
use bevy_log::debug;
use crate::{
    gui_framework::components::InteractionState,
    widgets::{
        blueprint::{StyleConfig, StateStyles, StyleOverrides},
        components::{Widget, WidgetStyle},
    },
};

/// Component to store the resolved style for a widget
#[derive(Component, Debug, Clone)]
pub struct ResolvedStyle {
    /// The final computed style after applying all cascading rules
    pub style: StyleConfig,
    /// Tracks if the style needs to be updated due to state changes
    pub dirty: bool,
}

impl ResolvedStyle {
    pub fn new(style: StyleConfig) -> Self {
        Self {
            style,
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

/// Event fired when a widget's resolved style changes
#[derive(Event, Debug, Clone)]
pub struct StyleChanged {
    pub entity: Entity,
    pub old_style: StyleConfig,
    pub new_style: StyleConfig,
}

/// System that resolves styles based on interaction states
/// This implements the cascading resolution: base → class → state → override
pub fn style_resolution_system(
    mut commands: Commands,
    mut style_changed_events: EventWriter<StyleChanged>,
    // Query widgets with interaction states
    mut widget_query: Query<
        (Entity, &Widget, &InteractionState, Option<&mut ResolvedStyle>),
        Or<(Changed<InteractionState>, Added<InteractionState>)>
    >,
    // Query all widgets that might need initial style resolution
    widget_init_query: Query<(Entity, &Widget), (Without<ResolvedStyle>, Without<InteractionState>)>,
) {
    // Handle widgets with interaction state changes
    for (entity, widget, interaction_state, resolved_style) in widget_query.iter_mut() {
        let base_style = &widget.blueprint.style;
        
        // Resolve style based on current interaction state
        let new_style = resolve_style_for_state(
            base_style,
            interaction_state.hovered,
            interaction_state.pressed,
            interaction_state.focused,
            false, // TODO: Add disabled state to InteractionState
        );

        if let Some(mut resolved) = resolved_style {
            // Check if style actually changed
            if !styles_equal(&resolved.style, &new_style) {
                let old_style = resolved.style.clone();
                resolved.style = new_style.clone();
                resolved.mark_dirty();
                
                debug!("Style changed for entity {:?}", entity);
                style_changed_events.send(StyleChanged {
                    entity,
                    old_style,
                    new_style,
                });
            }
        } else {
            // First time resolution
            commands.entity(entity).insert(ResolvedStyle::new(new_style.clone()));
            debug!("Initial style resolved for entity {:?}", entity);
        }
    }

    // Handle widgets without interaction states (static widgets)
    for (entity, widget) in widget_init_query.iter() {
        let style = widget.blueprint.style.clone();
        commands.entity(entity).insert(ResolvedStyle::new(style));
        debug!("Static style resolved for entity {:?}", entity);
    }
}

/// System that updates widget style components when resolved styles change
pub fn apply_resolved_styles_system(
    mut widget_style_query: Query<(&mut WidgetStyle, &mut ResolvedStyle), Changed<ResolvedStyle>>,
) {
    for (mut widget_style, mut resolved_style) in widget_style_query.iter_mut() {
        if resolved_style.is_dirty() {
            // Update the widget style component from resolved style
            update_widget_style_from_config(&mut widget_style, &resolved_style.style);
            resolved_style.clear_dirty();
            debug!("Applied resolved style to widget style component");
        }
    }
}

/// Resolve style based on interaction state using cascading rules
fn resolve_style_for_state(
    base_style: &StyleConfig,
    hovered: bool,
    pressed: bool,
    focused: bool,
    disabled: bool,
) -> StyleConfig {
    // Start with base style
    let mut resolved = base_style.clone();

    // Apply state-specific overrides if they exist
    if let Some(ref state_styles) = base_style.states {
        if let Some(state_override) = state_styles.get_for_state(hovered, pressed, focused, disabled) {
            resolved = state_override.apply_to(&resolved);
        }
    }

    resolved
}

/// Compare two StyleConfig instances for equality
fn styles_equal(a: &StyleConfig, b: &StyleConfig) -> bool {
    a.background_color == b.background_color &&
    a.border_color == b.border_color &&
    a.border_width == b.border_width &&
    a.border_radius == b.border_radius &&
    a.text_color == b.text_color &&
    a.text_size == b.text_size &&
    a.opacity == b.opacity
    // Note: We don't compare states as they don't affect the resolved style
}

/// Update WidgetStyle component from StyleConfig
fn update_widget_style_from_config(widget_style: &mut WidgetStyle, style_config: &StyleConfig) {
    if let Some(ref bg_color) = style_config.background_color {
        widget_style.background_color = Some(bg_color.to_color());
    }
    
    if let Some(ref border_color) = style_config.border_color {
        widget_style.border_color = Some(border_color.to_color());
    }
    
    if let Some(border_width) = style_config.border_width {
        widget_style.border_width = Some(border_width);
    }
    
    if let Some(border_radius) = style_config.border_radius {
        widget_style.border_radius = Some(border_radius);
    }
    
    if let Some(ref text_color) = style_config.text_color {
        widget_style.text_color = Some(text_color.to_color());
    }
    
    if let Some(text_size) = style_config.text_size {
        widget_style.text_size = Some(text_size);
    }
    
    if let Some(opacity) = style_config.opacity {
        widget_style.opacity = Some(opacity);
    }
}

/// System for debugging style resolution
pub fn style_resolution_debug_system(
    mut style_events: EventReader<StyleChanged>,
) {
    for event in style_events.read() {
        debug!("🎨 STYLE CHANGED: Entity {:?}", event.entity);
        
        // Log specific changes
        if event.old_style.background_color != event.new_style.background_color {
            debug!("  Background color: {:?} -> {:?}", 
                event.old_style.background_color, 
                event.new_style.background_color
            );
        }
        
        if event.old_style.opacity != event.new_style.opacity {
            debug!("  Opacity: {:?} -> {:?}", 
                event.old_style.opacity, 
                event.new_style.opacity
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use crate::widgets::blueprint::{ColorDef, StateStyles, StyleOverrides};

    #[test]
    fn test_style_resolution_no_states() {
        let base_style = StyleConfig {
            background_color: Some(ColorDef::Named("blue".to_string())),
            opacity: Some(1.0),
            ..Default::default()
        };

        let resolved = resolve_style_for_state(&base_style, false, false, false, false);
        
        assert_eq!(resolved.background_color, base_style.background_color);
        assert_eq!(resolved.opacity, base_style.opacity);
    }

    #[test]
    fn test_style_resolution_with_hover() {
        let mut base_style = StyleConfig {
            background_color: Some(ColorDef::Named("blue".to_string())),
            opacity: Some(1.0),
            ..Default::default()
        };

        let hover_override = StyleOverrides {
            background_color: Some(ColorDef::Named("lightblue".to_string())),
            opacity: Some(0.8),
            ..Default::default()
        };

        base_style.states = Some(StateStyles {
            hover: Some(hover_override),
            ..Default::default()
        });

        let resolved = resolve_style_for_state(&base_style, true, false, false, false);
        
        assert_eq!(resolved.background_color, Some(ColorDef::Named("lightblue".to_string())));
        assert_eq!(resolved.opacity, Some(0.8));
    }

    #[test]
    fn test_style_priority_pressed_over_hover() {
        let mut base_style = StyleConfig {
            background_color: Some(ColorDef::Named("blue".to_string())),
            ..Default::default()
        };

        let hover_override = StyleOverrides {
            background_color: Some(ColorDef::Named("lightblue".to_string())),
            ..Default::default()
        };

        let pressed_override = StyleOverrides {
            background_color: Some(ColorDef::Named("darkblue".to_string())),
            ..Default::default()
        };

        base_style.states = Some(StateStyles {
            hover: Some(hover_override),
            pressed: Some(pressed_override),
            ..Default::default()
        });

        // When both hovered and pressed, pressed should take priority
        let resolved = resolve_style_for_state(&base_style, true, true, false, false);
        
        assert_eq!(resolved.background_color, Some(ColorDef::Named("darkblue".to_string())));
    }

    #[test]
    fn test_styles_equal() {
        let style1 = StyleConfig {
            background_color: Some(ColorDef::Named("blue".to_string())),
            opacity: Some(1.0),
            ..Default::default()
        };

        let style2 = StyleConfig {
            background_color: Some(ColorDef::Named("blue".to_string())),
            opacity: Some(1.0),
            ..Default::default()
        };

        let style3 = StyleConfig {
            background_color: Some(ColorDef::Named("red".to_string())),
            opacity: Some(1.0),
            ..Default::default()
        };

        assert!(styles_equal(&style1, &style2));
        assert!(!styles_equal(&style1, &style3));
    }

    #[test]
    fn test_resolved_style_component() {
        let style = StyleConfig::default();
        let mut resolved = ResolvedStyle::new(style);

        assert!(resolved.is_dirty());
        resolved.clear_dirty();
        assert!(!resolved.is_dirty());
        
        resolved.mark_dirty();
        assert!(resolved.is_dirty());
    }
}

impl Default for StyleOverrides {
    fn default() -> Self {
        Self {
            background_color: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_color: None,
            text_size: None,
            opacity: None,
        }
    }
}

impl Default for StateStyles {
    fn default() -> Self {
        Self {
            hover: None,
            pressed: None,
            focused: None,
            disabled: None,
        }
    }
}