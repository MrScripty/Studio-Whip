use bevy_ecs::prelude::*;
use bevy_input::{keyboard::{KeyCode, KeyboardInput, Key}, ButtonInput};
use bevy_log::info;
use cosmic_text::{Editor, Motion, Action, CacheKey, CacheKeyFlags, Edit};
use bevy_math::Vec2;
use yrs::{Transact, Text};
use similar::{ChangeTag, TextDiff};

use crate::{
    YrsDocResource,
    gui_framework::{
        components::{Focus, CursorState, TextSelection, TextBufferCache, EditableText, TextLayoutOutput, PositionedGlyph},
        events::YrsTextChanged,
        interaction::utils::{global_to_local_cursor, cosmic_cursor_to_global_index},
    },
    FontServerResource,
    GlyphAtlasResource,
    SwashCacheResource,
};

pub(crate) fn text_editing_system(
    mut commands: Commands,
    mut keyboard_input_events: EventReader<KeyboardInput>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut focused_query: Query<(Entity, &mut CursorState, &mut TextSelection, &mut TextBufferCache), (With<Focus>, With<EditableText>)>,
    yrs_doc_res: Res<YrsDocResource>,
    mut yrs_text_changed_writer: EventWriter<YrsTextChanged>,
    font_system_res: Res<FontServerResource>,
    glyph_atlas_res: Res<GlyphAtlasResource>,
    swash_cache_res: Res<SwashCacheResource>,
    vk_context_res: Res<crate::VulkanContextResource>,
) {
    let Ok((entity, mut cursor_state, mut selection, mut text_cache)) = focused_query.get_single_mut() else {
        keyboard_input_events.clear();
        return;
    };

    let mut action_taken = false;
    let shift_pressed = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // --- Determine if any modification action should be taken ---
    let mut cosmic_action: Option<Action> = None;
    if keyboard_input.just_pressed(KeyCode::Backspace) { cosmic_action = Some(Action::Backspace); }
    else if keyboard_input.just_pressed(KeyCode::Delete) { cosmic_action = Some(Action::Delete); }
    else if keyboard_input.just_pressed(KeyCode::Enter) { cosmic_action = Some(Action::Enter); }
    else if keyboard_input.just_pressed(KeyCode::Tab) {
        cosmic_action = Some(if shift_pressed { Action::Unindent } else { Action::Indent });
    }

    let character_events: Vec<KeyboardInput> = keyboard_input_events.read().cloned().collect();
    let is_char_insertion = character_events.iter().any(|ev| matches!(ev.logical_key, Key::Character(_)) && ev.state.is_pressed());

    // --- Main Logic Branching: Modification vs. Navigation ---

    if cosmic_action.is_some() || is_char_insertion {
        // --- Path 1: Unified Content Modification ---
        action_taken = true;

        // 1. SETUP: Prepare the editor and capture the initial state.
        let Some(buffer) = text_cache.buffer.as_mut() else { return; };
        let mut font_system_guard = font_system_res.0.lock().unwrap();
        let mut editor = Editor::new(buffer);
        let current_cursor = editor.with_buffer(|b| global_to_local_cursor(b, cursor_state.position));
        editor.set_cursor(current_cursor);

        let text_before = editor.with_buffer(|b| {
            b.lines.iter().map(|line| line.text()).collect::<Vec<&str>>().join("\n")
        });

        // 2. PERFORM ACTION: Execute the specific modification.
        if let Some(action) = cosmic_action {
            editor.action(&mut font_system_guard.font_system, action);
        } else if is_char_insertion {
            for event in character_events.iter() {
                if let Key::Character(chars) = &event.logical_key {
                    if event.state.is_pressed() {
                        editor.insert_string(chars, None);
                    }
                }
            }
        }

        // 3. FINALIZE LOCAL STATE: Shape the buffer to apply layout changes.
        editor.shape_as_needed(&mut font_system_guard.font_system, true);

        let text_after = editor.with_buffer(|b| {
            b.lines.iter().map(|line| line.text()).collect::<Vec<&str>>().join("\n")
        });

        // 4. UPDATE VISUALS & LOGICAL STATE: Apply changes to all relevant components.
        // This block is now universal for all modifications.
        let new_cursor = editor.cursor();
        let new_cursor_pos = editor.with_buffer(|b| cosmic_cursor_to_global_index(b, new_cursor));
        cursor_state.position = new_cursor_pos;
        cursor_state.line = new_cursor.line;
        editor.with_buffer(|b| {
            if let Some(run) = b.layout_runs().find(|r| r.line_i == new_cursor.line) {
                let glyph_at_cursor = run.glyphs.iter().find(|g| g.start == new_cursor.index);
                let new_x = glyph_at_cursor.map_or_else(|| run.glyphs.first().map_or(0.0, |g| g.x) + run.line_w, |g| g.x);
                cursor_state.x_goal = Some(new_x as i32);
            } else {
                cursor_state.x_goal = None;
            }
        });
        selection.start = new_cursor_pos;
        selection.end = new_cursor_pos;

        let mut positioned_glyphs = Vec::new();
        let mut swash_cache = swash_cache_res.0.lock().unwrap();
        let mut glyph_atlas = glyph_atlas_res.0.lock().unwrap();
        let (device, queue, command_pool, allocator) = {
            let vk_guard = vk_context_res.0.lock().unwrap();
            (vk_guard.device.as_ref().unwrap().clone(), vk_guard.queue.unwrap(), vk_guard.command_pool.unwrap(), vk_guard.allocator.as_ref().unwrap().clone())
        };
        editor.with_buffer(|b| {
            for run in b.layout_runs() {
                let baseline_y = -run.line_y;
                for layout_glyph in run.glyphs.iter() {
                    let (cache_key, _, _) = CacheKey::new(layout_glyph.font_id, layout_glyph.glyph_id, layout_glyph.font_size, (layout_glyph.x, layout_glyph.y), CacheKeyFlags::empty());
                    let Some(swash_image) = swash_cache.get_image(&mut font_system_guard.font_system, cache_key) else { continue; };
                    if let Ok(glyph_info) = glyph_atlas.add_glyph(&device, queue, command_pool, &allocator, cache_key, &swash_image) {
                        let placement = swash_image.placement;
                        let width = placement.width as f32;
                        let height = placement.height as f32;
                        let top_left = Vec2::new(layout_glyph.x, baseline_y + placement.top as f32);
                        let top_right = Vec2::new(layout_glyph.x + width, baseline_y + placement.top as f32);
                        let bottom_right = Vec2::new(layout_glyph.x + width, baseline_y + placement.top as f32 - height);
                        let bottom_left = Vec2::new(layout_glyph.x, baseline_y + placement.top as f32 - height);
                        positioned_glyphs.push(PositionedGlyph { glyph_info: *glyph_info, layout_glyph: layout_glyph.clone(), vertices: [top_left, top_right, bottom_right, bottom_left] });
                    }
                }
            }
        });
        commands.entity(entity).insert(TextLayoutOutput { glyphs: positioned_glyphs });

        // 5. COMMIT TO TRUTH & SIGNAL: Apply the calculated diff to YRS and send the event.
        if let Some(yrs_text) = yrs_doc_res.text_map.lock().unwrap().get(&entity) {
            let mut txn = yrs_doc_res.doc.transact_mut();
            let diff = TextDiff::from_chars(&text_before, &text_after);
            let mut index = 0;
            for change in diff.iter_all_changes() {
                match change.tag() {
                    ChangeTag::Delete => { yrs_text.remove_range(&mut txn, index, change.value().len() as u32); }
                    ChangeTag::Insert => { yrs_text.insert(&mut txn, index, change.value()); index += change.value().len() as u32; }
                    ChangeTag::Equal => { index += change.value().len() as u32; }
                }
            }
        }
        yrs_text_changed_writer.send(YrsTextChanged { entity });

    } else {
        // --- Path 2: Navigation ---
        // This logic remains unchanged.
        let mut motion_to_perform: Option<(Motion, bool)> = None;
        if keyboard_input.just_pressed(KeyCode::ArrowLeft) { motion_to_perform = Some((Motion::Left, false)); }
        if keyboard_input.just_pressed(KeyCode::ArrowRight) { motion_to_perform = Some((Motion::Right, false)); }
        if keyboard_input.just_pressed(KeyCode::ArrowUp) { motion_to_perform = Some((Motion::Up, true)); }
        if keyboard_input.just_pressed(KeyCode::ArrowDown) { motion_to_perform = Some((Motion::Down, true)); }

        if let Some((motion, is_vertical)) = motion_to_perform {
            if let Some(original_buffer) = text_cache.buffer.as_ref() {
                let mut new_cursor_opt: Option<cosmic_text::Cursor> = None;
                if is_vertical {
                    if let Some(origin_x) = cursor_state.x_goal {
                        let target_line_index = if motion == Motion::Up { cursor_state.line.saturating_sub(1) } else { cursor_state.line + 1 };
                        if let Some(target_run) = original_buffer.layout_runs().find(|r| r.line_i == target_line_index) {
                            let target_y = target_run.line_top + (target_run.line_height / 2.0);
                            new_cursor_opt = original_buffer.hit(origin_x as f32, target_y);
                        }
                    }
                } else {
                    let start_cursor = global_to_local_cursor(original_buffer, cursor_state.position);
                    let mut temp_buffer = original_buffer.clone();
                    let mut font_system = font_system_res.0.lock().unwrap();
                    if let Some((new_cursor, _)) = temp_buffer.cursor_motion(&mut font_system.font_system, start_cursor, None, motion) {
                        new_cursor_opt = Some(new_cursor);
                    }
                }
                if let Some(new_cursor) = new_cursor_opt {
                    action_taken = true;
                    let new_global_pos = cosmic_cursor_to_global_index(original_buffer, new_cursor);
                    cursor_state.position = new_global_pos;
                    cursor_state.line = new_cursor.line;
                    let new_run = original_buffer.layout_runs().find(|r| r.line_i == new_cursor.line).unwrap();
                    let glyph_at_cursor = new_run.glyphs.iter().find(|g| g.start == new_cursor.index);
                    let new_x = glyph_at_cursor.map_or_else(|| new_run.glyphs.first().map_or(0.0, |g| g.x) + new_run.line_w, |g| g.x);
                    cursor_state.x_goal = Some(new_x as i32);
                    if shift_pressed { selection.end = new_global_pos; } else { selection.start = new_global_pos; selection.end = new_global_pos; }
                }
            }
        }
    }

    if action_taken {
        info!( "Input Action: Entity {:?}, New Cursor [L{}, P{}, X-Goal: {:?}], New Selection [{}, {}]", entity, cursor_state.line, cursor_state.position, cursor_state.x_goal, selection.start, selection.end );
    }
    if action_taken {
        keyboard_input_events.clear();
    }
}