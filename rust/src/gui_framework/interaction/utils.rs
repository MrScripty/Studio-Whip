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

/// Converts a `cosmic_text::Cursor` (line index and byte offset within the line)
/// into a global byte offset within the entire text buffer.
///
/// # Arguments
///
/// * `buffer` - The `cosmic_text::Buffer` containing the laid-out text.
/// * `cursor` - The `cosmic_text::Cursor` to convert.
///
/// # Returns
///
/// * `usize` representing the global byte offset from the beginning of the text.
pub fn cosmic_cursor_to_global_index(buffer: &Buffer, cursor: Cursor) -> usize {
    let mut global_index = 0;
    for (i, line) in buffer.lines.iter().enumerate() {
        if i < cursor.line {
            // Add the length of the line's text plus 1 for the newline character.
            // This is correct for all but the last line, but the loop condition handles that.
            global_index += line.text().len() + 1;
        } else {
            // We are on the target line. Add the cursor's index within this line and break.
            global_index += cursor.index;
            break;
        }
    }
    global_index
}