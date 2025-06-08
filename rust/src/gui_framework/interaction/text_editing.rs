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
pub(crate) fn text_editing_system(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut char_events: EventReader<KeyboardInput>,
    mut focused_query: Query<(Entity, &mut CursorState, &mut TextSelection, &TextBufferCache), (With<Focus>, With<EditableText>)>,
    yrs_doc_res: Res<YrsDocResource>,
    mut yrs_text_changed_writer: EventWriter<YrsTextChanged>,
    mut font_system_res: ResMut<crate::FontServerResource>, // Added FontServerResource
) {
    for (entity, mut cursor_state, mut selection, text_cache) in focused_query.iter_mut() {
        let yrs_doc = yrs_doc_res.doc.clone();
        let text_map_guard = yrs_doc_res.text_map.lock().unwrap();
        let Some(yrs_text_ref) = text_map_guard.get(&entity) else {
            warn!("TextEditing: YrsTextRef not found for focused entity {:?}", entity);
            continue;
        };
        let yrs_text = yrs_text_ref.clone(); // Clone to release lock sooner
        drop(text_map_guard);

        let Some(buffer) = text_cache.buffer.as_ref() else {
            warn!("TextEditing: TextBufferCache is None for focused entity {:?}", entity);
            continue;
        };

        // Create a temporary editor for this operation
        // Note: We are not using cosmic-text's full Editor state persistence here,
        // as YRS is the source of truth. We use the Editor for its editing methods.
        let mut editor = Editor::new(buffer.clone());
        editor.set_cursor(cosmic_text::Cursor::new(cursor_state.line, cursor_state.position));
        editor.set_selection(cosmic_text::Selection::Normal(cosmic_text::Cursor::new(cursor_state.line, selection.start.min(selection.end))));

        let mut changed = false;
        let mut new_cursor_pos = cursor_state.position;
        let mut new_selection_start = selection.start;
        let mut new_selection_end = selection.end;

        let shift_pressed = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        // --- Handle Deletion ---
        if keyboard_input.just_pressed(KeyCode::Backspace) {
            if selection.start != selection.end { // If there's a selection, delete it
                let (del_start, del_end) = (min(selection.start, selection.end), max(selection.start, selection.end));
                yrs_text.remove_range(&mut yrs_doc.transact_mut(), del_start as u32, (del_end - del_start) as u32);
                new_cursor_pos = del_start;
                new_selection_start = new_cursor_pos;
                new_selection_end = new_cursor_pos;
            } else if cursor_state.position > 0 { // If no selection, delete char before cursor
                yrs_text.remove_range(&mut yrs_doc.transact_mut(), (cursor_state.position - 1) as u32, 1);
                new_cursor_pos = cursor_state.position - 1;
                new_selection_start = new_cursor_pos;
                new_selection_end = new_cursor_pos;
            }
            changed = true;
        } else if keyboard_input.just_pressed(KeyCode::Delete) {
            if selection.start != selection.end { // If there's a selection, delete it
                let (del_start, del_end) = (min(selection.start, selection.end), max(selection.start, selection.end));
                yrs_text.remove_range(&mut yrs_doc.transact_mut(), del_start as u32, (del_end - del_start) as u32);
                new_cursor_pos = del_start;
                new_selection_start = new_cursor_pos;
                new_selection_end = new_cursor_pos;
            } else { // If no selection, delete char after cursor
                let current_len = yrs_text.len(&yrs_doc.transact_mut()) as usize;
                if cursor_state.position < current_len {
                    yrs_text.remove_range(&mut yrs_doc.transact_mut(), cursor_state.position as u32, 1);
                    // Cursor position doesn't change, but selection should collapse
                    new_selection_start = new_cursor_pos;
                    new_selection_end = new_cursor_pos;
                }
            }
            changed = true;
        }

        // --- Handle Character Input ---
        for event in char_events.read() {
            if let Key::Character(chars) = &event.logical_key {
                if !event.repeat && event.state.is_pressed() {
                    let mut txn = yrs_doc.transact_mut(); // Make txn mutable
                    if selection.start != selection.end { // If selection, replace it
                        let (replace_start, replace_end) = (min(selection.start, selection.end), max(selection.start, selection.end));
                        yrs_text.remove_range(&mut txn, replace_start as u32, (replace_end - replace_start) as u32);
                        yrs_text.insert(&mut txn, replace_start as u32, chars);
                        new_cursor_pos = replace_start + chars.len();
                    } else { // No selection, insert at cursor
                        yrs_text.insert(&mut txn, cursor_state.position as u32, chars);
                        new_cursor_pos = cursor_state.position + chars.len();
                    }
                    new_selection_start = new_cursor_pos;
                    new_selection_end = new_cursor_pos;
                    changed = true;
                }
            }
        }
        char_events.clear(); // Consume events

        // --- Handle Arrow Key Navigation/Selection ---
        // We use the editor's action method for cursor motion, then update our state.
        // This is simpler than re-implementing all cursor motion logic.
        let mut font_system_guard = font_system_res.0.lock().unwrap();
        let mut action_taken = false;
        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            editor.action(&mut font_system_guard.font_system, cosmic_text::Action::Motion(Motion::Left));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            editor.action(&mut font_system_guard.font_system, cosmic_text::Action::Motion(Motion::Right));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            editor.action(&mut font_system_guard.font_system, cosmic_text::Action::Motion(Motion::Up));
            action_taken = true;
        }
        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            editor.action(&mut font_system_guard.font_system, cosmic_text::Action::Motion(Motion::Down));
            action_taken = true;
        }

        drop(font_system_guard); // Release lock
        if action_taken {
            let editor_cursor = editor.cursor();
            new_cursor_pos = editor_cursor.index;
            // If shift is pressed, extend selection. Otherwise, collapse it.
            if shift_pressed {
                new_selection_end = new_cursor_pos;
                // new_selection_start remains the original selection.start
            } else {
                new_selection_start = new_cursor_pos;
                new_selection_end = new_cursor_pos;
            }
            changed = true; // Cursor or selection definitely changed
        }


        // --- Update Components if anything changed ---
        if changed {
            // Update CursorState
            // To get the correct line after edits, we need to re-layout or use cosmic-text's editor state.
            // For now, we'll just update the position. Line update will happen after re-layout.
            // A more robust solution would involve getting the line from the editor after actions.
            let (final_cursor_line, final_cursor_pos) = if action_taken {
                // If an arrow key action was taken, the editor's cursor is most up-to-date
                (editor.cursor().line, editor.cursor().index)
            } else {
                // For typing/deletion, we calculated new_cursor_pos. Line needs update.
                // This is a simplification; a full re-layout would be needed for accurate line.
                // For now, assume line doesn't change drastically or will be corrected by layout.
                (cursor_state.line, new_cursor_pos)
            };

            cursor_state.position = final_cursor_pos;
            cursor_state.line = final_cursor_line; // This might be stale until next layout

            selection.start = new_selection_start;
            selection.end = new_selection_end;

            info!(
                "Text Edited: Entity {:?}, New Cursor [L{}, P{}], New Selection [{}, {}]",
                entity, cursor_state.line, cursor_state.position, selection.start, selection.end
            );
            yrs_text_changed_writer.send(YrsTextChanged { entity });
        }
    }
}