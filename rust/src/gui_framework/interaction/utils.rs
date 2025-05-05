use bevy_math::Vec2;
use cosmic_text::{Buffer, Cursor};

/// Calculates the text cursor position (line and byte index) corresponding
/// to a given local coordinate within the text bounds.
///
/// # Arguments
///
/// * `buffer` - The `cosmic_text::Buffer` containing the laid-out text.
/// * `local_pos_ydown` - The position to check, in the text entity's local
///   coordinate system where the Y-axis points downwards (matching cosmic-text).
///
/// # Returns
///
/// * `Some(Cursor)` containing the line index and byte offset if the position hits text.
/// * `None` if the position is outside the text bounds.
pub fn get_cursor_at_position(buffer: &Buffer, local_pos_ydown: Vec2) -> Option<Cursor> {
    buffer.hit(local_pos_ydown.x, local_pos_ydown.y)
}