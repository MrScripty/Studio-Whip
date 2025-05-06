use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window, CursorMoved};
use bevy_transform::prelude::GlobalTransform;
use bevy_math::{Vec2, Affine3A};

use crate::gui_framework::{
    components::{Focus, TextSelection, TextBufferCache},
    interaction::utils::get_cursor_at_position,
};
use super::super::plugins::interaction::MouseContext; // Access MouseContext defined in interaction plugin

/// System responsible for handling text selection via dragging.
/// Runs only when the mouse context is `TextInteraction`.
pub(crate) fn text_drag_selection_system(
    // Input resources & events
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_context: Res<MouseContext>,
    // Use ParamSet for conflicting TextSelection access
    mut text_queries: ParamSet<(
        // p0: Query for focused entity
        Query<(Entity, &GlobalTransform, &TextBufferCache), With<Focus>>,
        // p1: Query for mutable access to TextSelection (used after finding the entity)
        Query<&mut TextSelection>,
    )>,
) {
    // Only run if we are in the text interaction context
    if mouse_context.context != super::super::plugins::interaction::MouseContextType::TextInteraction {
        cursor_moved_events.clear(); // Consume events if not relevant
        return;
    }

    let Ok(primary_window) = windows.get_single() else { return; };
    let window_height = primary_window.height();

    // Get the latest cursor position from the events
    let latest_cursor_pos_window = cursor_moved_events.read().last().map(|e| e.position);
    cursor_moved_events.clear(); // Consume events

    if let Some(cursor_pos_window) = latest_cursor_pos_window {
        let cursor_pos_world = Vec2::new(cursor_pos_window.x, window_height - cursor_pos_window.y);

        // Find the focused entity
        // Use p0 (immutable query) to find the focused entity first
        if let Ok((focused_entity, transform, text_cache)) = text_queries.p0().get_single() {
            // Clone necessary data because we can't hold the immutable borrow from p0
            // while trying to get a mutable borrow from p1 later.
            let focused_entity_id = focused_entity; // Clone Entity ID
            let buffer_option = text_cache.buffer.clone(); // Clone the Option<Buffer> Arc if needed, or just check existence

            let Some(buffer) = buffer_option.as_ref() else { return; }; // Need the buffer cache
            // Transform cursor world position to entity's local space (Y-up) using cloned transform
            let inverse_transform: Affine3A = transform.affine().inverse();
            let cursor_pos_local_yup = inverse_transform.transform_point3(cursor_pos_world.extend(0.0)).truncate();

            // Convert to local Y-down for cosmic-text
            let cursor_pos_local_ydown = Vec2::new(cursor_pos_local_yup.x, -cursor_pos_local_yup.y);

            // Use the utility function to find the character index under the cursor
            if let Some(hit_cursor) = get_cursor_at_position(buffer, cursor_pos_local_ydown) {
                let new_end_pos = hit_cursor.index;

                // Get mutable access to the selection component using p1 and the cloned entity ID
                if let Ok(mut selection) = text_queries.p1().get_mut(focused_entity_id) {

                    // Only update if the end position actually changes
                    if selection.end != new_end_pos {
                        selection.end = new_end_pos;
                        // info!("Drag selection updated: Entity {:?}, Selection [{}, {}]", focused_entity, selection.start, selection.end);
                    }
                } else {
                    // Component not found (likely deferred command application). Do nothing.
                }
            }
            // If hit_cursor is None (cursor moved outside text bounds during drag),
            // we could potentially clamp the selection end to the start/end of the text,
            // but for now, we just stop updating the selection end.
        }
        // If no entity has focus, do nothing.
    }
}