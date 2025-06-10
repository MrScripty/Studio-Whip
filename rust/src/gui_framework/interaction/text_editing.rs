use bevy_ecs::prelude::*;
use bevy_input::{keyboard::{KeyCode, KeyboardInput, Key}, ButtonInput};
use bevy_log::{info, warn};
use cosmic_text::{Attrs, BufferRef, Edit, Editor, FontSystem, Metrics, Motion, Shaping};
use std::cmp::{min, max};
use yrs::{Transact, Text};

use crate::{
    YrsDocResource,
    gui_framework::{
        components::{Focus, CursorState, TextSelection, TextBufferCache, EditableText},
        events::YrsTextChanged,
    },
};

/// System responsible for handling keyboard input for text editing operations
/// like typing, deletion, and selection modification with arrow keys.
/// 1. Initialize cosmic-text Editor from Bevy component state.
/// 2. Perform actions (either on YRS data or directly on the Editor).
/// 3. If YRS data changed, re-sync the Editor's internal buffer.
/// 4. Read the final state from the Editor.
/// 5. Update Bevy components from the Editor's final state.
pub(crate) fn text_editing_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    // Make the TextBufferCache query mutable as proposed.
    mut focused_query: Query<(Entity, &mut CursorState, &mut TextSelection, &mut TextBufferCache), (With<Focus>, With<EditableText>)>,
    yrs_doc_res: Res<YrsDocResource>,
    mut yrs_text_changed_writer: EventWriter<YrsTextChanged>,
    mut font_system_res: ResMut<FontServerResource>,
) {
    let Ok((entity, mut cursor_state, mut selection, mut text_cache)) = focused_query.get_single_mut() else {
        // No focused entity, or more than one. Clear events and do nothing.
        keyboard_input_events.clear();
        return;
    };

    // --- 1. INITIAL SETUP ---
    let yrs_doc = yrs_doc_res.doc.clone();
    let Some(yrs_text) = yrs_doc_res.text_map.lock().unwrap().get(&entity).cloned() else {
        warn!("TextEditing: YrsTextRef not found for focused entity {:?}", entity);
        return;
    };

    let Some(buffer) = text_cache.buffer.as_mut() else {
        warn!("TextEditing: TextBufferCache is None for focused entity {:?}", entity);
        return;
    };

    // Create a mutable `cosmic_text::Editor` from our buffer.
    let mut editor = Editor::new(buffer);

    // Set the editor's cursor and selection from our Bevy components.
    let initial_cosmic_cursor = global_to_local_cursor(editor.buffer_ref(), cursor_state.position);
    editor.set_cursor(initial_cosmic_cursor);

    let selection_start_cursor = global_to_local_cursor(editor.buffer_ref(), selection.start);
    editor.set_selection(cosmic_text::Selection::Normal(selection_start_cursor));
    // Now, perform a motion to the end of the selection to set it correctly in the editor.
    let selection_end_cursor = global_to_local_cursor(editor.buffer_ref(), selection.end);
    editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::MoveCursor(selection_end_cursor, true));


    // --- 2. HANDLE INPUT AND MODIFY STATE ---
    let mut content_changed = false;
    let mut action_taken = false;
    let mut new_global_cursor_pos = cursor_state.position;

    let shift_pressed = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // --- Handle Deletion ---
    if keyboard_input.just_pressed(KeyCode::Backspace) {
        if editor.delete_selection() {
            // Selection was deleted by the editor.
        } else {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Backspace);
        }
        content_changed = true;
        action_taken = true;
    } else if keyboard_input.just_pressed(KeyCode::Delete) {
        if editor.delete_selection() {
            // Selection was deleted by the editor.
        } else {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Delete);
        }
        content_changed = true;
        action_taken = true;
    }

    // --- Handle Character Input ---
    for event in keyboard_input_events.read() {
        if let Key::Character(chars) = &event.logical_key {
            if !event.repeat && event.state.is_pressed() {
                editor.insert_string(chars, None);
                content_changed = true;
                action_taken = true;
            }
        }
    }

    // --- Handle Arrow Key Navigation/Selection ---
    if !content_changed { // Only process navigation if no text was changed
        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Motion(Motion::Left, shift_pressed));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Motion(Motion::Right, shift_pressed));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Motion(Motion::Up, shift_pressed));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            editor.action(font_system_res.0.lock().unwrap().font_system(), cosmic_text::Action::Motion(Motion::Down, shift_pressed));
            action_taken = true;
        }
    }

    // --- 3. SYNCHRONIZE YRS & EDITOR (The Key Fix) ---
    if content_changed {
        // Get the full text from the now-modified editor buffer.
        let mut editor_text = String::new();
        for line in editor.buffer_ref().lines.iter() {
            editor_text.push_str(line.text());
            editor_text.push('\n');
        }
        if !editor_text.is_empty() {
            editor_text.pop(); // Remove trailing newline
        }

        // Get the current text from YRS to compare.
        let yrs_text_string = yrs_text.get_string(&yrs_doc.transact_mut());

        // Replace the entire YRS text with the editor's text.
        // This is simpler and more robust than calculating diffs.
        // YRS is efficient at handling this.
        let mut txn = yrs_doc.transact_mut();
        yrs_text.remove_range(&mut txn, 0, yrs_text_string.len() as u32);
        yrs_text.insert(&mut txn, 0, &editor_text);

        // Send the event to notify other systems (like text_layout_system).
        yrs_text_changed_writer.send(YrsTextChanged { entity });
    }

    // --- 4. UPDATE BEVY COMPONENTS FROM FINAL EDITOR STATE ---
    if action_taken {
        // Get the final cursor and selection from the editor.
        let final_cursor = editor.cursor();
        let final_selection_bounds = editor.selection_bounds();

        // Convert back to global byte offsets.
        cursor_state.position = cosmic_cursor_to_global_index(editor.buffer_ref(), final_cursor);
        cursor_state.line = final_cursor.line;

        if let Some((start_cursor, end_cursor)) = final_selection_bounds {
            selection.start = cosmic_cursor_to_global_index(editor.buffer_ref(), start_cursor);
            selection.end = cosmic_cursor_to_global_index(editor.buffer_ref(), end_cursor);
        } else {
            // No selection, so collapse it to the cursor position.
            selection.start = cursor_state.position;
            selection.end = cursor_state.position;
        }

        info!(
            "Text Edited: Entity {:?}, New Cursor [L{}, P{}], New Selection [{}, {}]",
            entity, cursor_state.line, cursor_state.position, selection.start, selection.end
        );
    }
}