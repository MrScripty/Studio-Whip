use bevy_ecs::prelude::*;
use bevy_input::{keyboard::{KeyCode, KeyboardInput, Key}, ButtonInput};
use bevy_log::{info, warn};
use cosmic_text::{Attrs, Buffer, Edit, Editor, Metrics, Motion, Selection, Shaping};
use yrs::{Transact, Text, GetString};

use crate::{
    YrsDocResource,
    gui_framework::{
        components::{Focus, CursorState, TextSelection, TextBufferCache, EditableText},
        events::YrsTextChanged,
        interaction::utils::{global_to_local_cursor, cosmic_cursor_to_global_index},
    },
    FontServerResource,
};

pub(crate) fn text_editing_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut focused_query: Query<(Entity, &mut CursorState, &mut TextSelection, &TextBufferCache), (With<Focus>, With<EditableText>)>,
    yrs_doc_res: Res<YrsDocResource>,
    mut yrs_text_changed_writer: EventWriter<YrsTextChanged>,
    font_system_res: Res<FontServerResource>,
) {
    let Ok((entity, mut cursor_state, mut selection, text_cache)) = focused_query.get_single_mut() else {
        keyboard_input_events.clear();
        return;
    };

    let mut action_taken = false;
    let shift_pressed = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let is_content_modification = keyboard_input.just_pressed(KeyCode::Backspace)
        || keyboard_input.just_pressed(KeyCode::Delete)
        || keyboard_input_events.read().any(|ev| matches!(ev.logical_key, Key::Character(_)) && ev.state.is_pressed());

    if is_content_modification {
        // This logic for content modification remains unchanged.
        action_taken = true;
        let yrs_doc = yrs_doc_res.doc.clone();
        let Some(yrs_text) = yrs_doc_res.text_map.lock().unwrap().get(&entity).cloned() else { return; };
        let current_text = yrs_text.get_string(&yrs_doc.transact_mut());
        let mut font_system = font_system_res.0.lock().unwrap();
        let metrics = Metrics::new(text_cache.buffer.as_ref().unwrap().metrics().font_size, text_cache.buffer.as_ref().unwrap().metrics().line_height);
        let mut buffer = Buffer::new(&mut font_system.font_system, metrics);
        if let Some(original_buffer) = text_cache.buffer.as_ref() {
            buffer.set_size(&mut font_system.font_system, original_buffer.size().0, original_buffer.size().1);
            buffer.set_wrap(&mut font_system.font_system, original_buffer.wrap());
        }
        buffer.set_text(&mut font_system.font_system, &current_text, &Attrs::new(), Shaping::Advanced);
        let mut editor = Editor::new(&mut buffer);
        let selection_start_cursor = editor.with_buffer(|b| global_to_local_cursor(b, selection.start));
        let selection_end_cursor = editor.with_buffer(|b| global_to_local_cursor(b, selection.end));
        editor.set_selection(Selection::Normal(selection_start_cursor));
        editor.set_cursor(selection_end_cursor);
        if keyboard_input.just_pressed(KeyCode::Backspace) { if !editor.delete_selection() { editor.action(&mut font_system.font_system, cosmic_text::Action::Backspace); } }
        else if keyboard_input.just_pressed(KeyCode::Delete) { if !editor.delete_selection() { editor.action(&mut font_system.font_system, cosmic_text::Action::Delete); } }
        for event in keyboard_input_events.read() { if let Key::Character(chars) = &event.logical_key { if event.state.is_pressed() { editor.insert_string(chars, None); } } }
        let mut editor_text = String::new();
        editor.with_buffer(|b| { for line in b.lines.iter() { editor_text.push_str(line.text()); editor_text.push('\n'); } });
        if !editor_text.is_empty() { editor_text.pop(); }
        let mut txn = yrs_doc.transact_mut();
        yrs_text.remove_range(&mut txn, 0, current_text.len() as u32);
        yrs_text.insert(&mut txn, 0, &editor_text);
        yrs_text_changed_writer.send(YrsTextChanged { entity });
        editor.shape_as_needed(&mut font_system.font_system, true);
        let final_cursor = editor.cursor();
        let final_selection_bounds = editor.selection_bounds();
        cursor_state.position = editor.with_buffer(|b| cosmic_cursor_to_global_index(b, final_cursor));
        cursor_state.line = final_cursor.line;
        cursor_state.x_goal = None;
        if let Some((start_cursor, end_cursor)) = final_selection_bounds {
            selection.start = editor.with_buffer(|b| cosmic_cursor_to_global_index(b, start_cursor));
            selection.end = editor.with_buffer(|b| cosmic_cursor_to_global_index(b, end_cursor));
        } else {
            selection.start = cursor_state.position;
            selection.end = cursor_state.position;
        }
    } else {
        // --- Path 2: Handle Navigation (Arrow Keys) ---
        let mut motion_to_perform: Option<(Motion, bool)> = None;
        if keyboard_input.just_pressed(KeyCode::ArrowLeft) { motion_to_perform = Some((Motion::Left, false)); }
        if keyboard_input.just_pressed(KeyCode::ArrowRight) { motion_to_perform = Some((Motion::Right, false)); }
        if keyboard_input.just_pressed(KeyCode::ArrowUp) { motion_to_perform = Some((Motion::Up, true)); }
        if keyboard_input.just_pressed(KeyCode::ArrowDown) { motion_to_perform = Some((Motion::Down, true)); }

        if let Some((motion, is_vertical)) = motion_to_perform {
            if let Some(original_buffer) = text_cache.buffer.as_ref() {
                let mut new_cursor_opt: Option<cosmic_text::Cursor> = None;

                if is_vertical {
                    // --- NEW: Vertical Movement using buffer.hit() ---
                    // 1. Get the origin X coordinate from our stored x_goal.
                    if let Some(origin_x) = cursor_state.x_goal {
                        // 2. Determine the target line's Y coordinate.
                        let target_line_index = if motion == Motion::Up {
                            cursor_state.line.saturating_sub(1)
                        } else {
                            cursor_state.line + 1
                        };

                        if let Some(target_run) = original_buffer.layout_runs().find(|r| r.line_i == target_line_index) {
                            let target_y = target_run.line_top + (target_run.line_height / 2.0);
                            // 3. Use hit() to find the closest cursor on the target line.
                            new_cursor_opt = original_buffer.hit(origin_x as f32, target_y);
                        }
                    }
                } else {
                    // --- Horizontal Movement using cursor_motion (this is correct) ---
                    let start_cursor = global_to_local_cursor(original_buffer, cursor_state.position);
                    let mut temp_buffer = original_buffer.clone();
                    let mut font_system = font_system_res.0.lock().unwrap();
                    if let Some((new_cursor, _)) = temp_buffer.cursor_motion(&mut font_system.font_system, start_cursor, None, motion) {
                        new_cursor_opt = Some(new_cursor);
                    }
                }

                // --- Universal State Update ---
                if let Some(new_cursor) = new_cursor_opt {
                    action_taken = true;

                    // Update logical position
                    let new_global_pos = cosmic_cursor_to_global_index(original_buffer, new_cursor);
                    cursor_state.position = new_global_pos;
                    cursor_state.line = new_cursor.line;

                    // **CRITICAL FIX**: After any move, recalculate and update the x_goal to match the new position.
                    let new_run = original_buffer.layout_runs().find(|r| r.line_i == new_cursor.line).unwrap();
                    let glyph_at_cursor = new_run.glyphs.iter().find(|g| g.start == new_cursor.index);
                    let new_x = glyph_at_cursor.map_or_else(
                        || new_run.glyphs.first().map_or(0.0, |g| g.x) + new_run.line_w,
                        |g| g.x,
                    );
                    cursor_state.x_goal = Some(new_x as i32);

                    // Update selection
                    if shift_pressed {
                        selection.end = new_global_pos;
                    } else {
                        selection.start = new_global_pos;
                        selection.end = new_global_pos;
                    }
                }
            }
        }
    }

    if action_taken {
        info!( "Input Action: Entity {:?}, New Cursor [L{}, P{}, X-Goal: {:?}], New Selection [{}, {}]", entity, cursor_state.line, cursor_state.position, cursor_state.x_goal, selection.start, selection.end );
    }
    keyboard_input_events.clear();
}