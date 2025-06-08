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

/// Converts a global byte offset within a `cosmic_text::Buffer` into a
/// local `cosmic_text::Cursor` (line index and byte offset within the line).
///
/// # Arguments
///
/// * `buffer` - The `cosmic_text::Buffer` containing the laid-out text.
/// * `global_pos` - The global byte offset from the beginning of the text.
///
/// # Returns
///
/// * `cosmic_text::Cursor` corresponding to the global position.
pub fn global_to_local_cursor(buffer: &Buffer, global_pos: usize) -> Cursor {
    let mut current_byte_offset = 0;
    for (line_i, line) in buffer.lines.iter().enumerate() {
        let line_byte_len = line.text().len();
        // The total length of the line in the original string includes the newline,
        // except for the very last line.
        let line_len_with_newline = if line_i < buffer.lines.len() - 1 {
            line_byte_len + 1
        } else {
            line_byte_len
        };

        if global_pos >= current_byte_offset && global_pos <= current_byte_offset + line_byte_len {
            // The position is on this line. The local index is the difference.
            return Cursor::new(line_i, global_pos - current_byte_offset);
        }
        current_byte_offset += line_len_with_newline;
    }

    // If the global position is beyond all text (e.g., clicking after the last character),
    // place the cursor at the end of the last line.
    if let Some(last_line) = buffer.lines.last() {
        Cursor::new(buffer.lines.len() - 1, last_line.text().len())
    } else {
        Cursor::new(0, 0)
    }
}