use bevy_ecs::prelude::*;
use bevy_input::{keyboard::{KeyCode, KeyboardInput, Key}, ButtonInput};
use bevy_log::{info, warn};
// Corrected imports based on compiler errors and new needs
use cosmic_text::{Editor, Motion, Action, CacheKey, CacheKeyFlags, Edit, Font, LayoutGlyph};
use swash::FontRef;
use bevy_math::Vec2;
use yrs::{Transact, Text};

use crate::{
    YrsDocResource,
    // Import all the types we need, as pointed out by the compiler
    gui_framework::{
        components::{Focus, CursorState, TextSelection, TextBufferCache, EditableText, TextLayoutOutput, PositionedGlyph},
        events::YrsTextChanged,
        interaction::utils::{global_to_local_cursor, cosmic_cursor_to_global_index},
    },
    FontServerResource,
    // Correctly import the resource wrappers
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

    let is_content_modification = keyboard_input.just_pressed(KeyCode::Backspace)
        || keyboard_input.just_pressed(KeyCode::Delete)
        || keyboard_input_events.read().any(|ev| matches!(ev.logical_key, Key::Character(_)) && ev.state.is_pressed());

    if is_content_modification {
        action_taken = true;

        let Some(buffer) = text_cache.buffer.as_mut() else {
            warn!("TextBufferCache is empty for focused entity {:?}. Cannot perform edit.", entity);
            return;
        };

        let mut font_system = font_system_res.0.lock().unwrap();
        let mut editor = Editor::new(buffer);

        let current_cursor = editor.with_buffer(|b| global_to_local_cursor(b, cursor_state.position));
        editor.set_cursor(current_cursor);

        let initial_cursor_pos = cursor_state.position;

        if keyboard_input.just_pressed(KeyCode::Backspace) {
            editor.action(&mut font_system.font_system, Action::Backspace);
            editor.shape_as_needed(&mut font_system.font_system, true);

            let new_cursor = editor.cursor();
            let new_cursor_pos = editor.with_buffer(|b| cosmic_cursor_to_global_index(b, new_cursor));

            cursor_state.position = new_cursor_pos;
            cursor_state.line = new_cursor.line;
            editor.with_buffer(|b| {
                if let Some(run) = b.layout_runs().find(|r| r.line_i == new_cursor.line) {
                    let glyph_at_cursor = run.glyphs.iter().find(|g| g.start == new_cursor.index);
                    let new_x = glyph_at_cursor.map_or_else(
                        || run.glyphs.first().map_or(0.0, |g| g.x) + run.line_w,
                        |g| g.x,
                    );
                    cursor_state.x_goal = Some(new_x as i32);
                } else {
                    cursor_state.x_goal = None;
                }
            });
            selection.start = new_cursor_pos;
            selection.end = new_cursor_pos;

            // *** THE FIX IS HERE (AGAIN) - A more faithful recreation of text_layout_system ***
            let mut positioned_glyphs = Vec::new();
            let mut swash_cache = swash_cache_res.0.lock().unwrap();
            let mut glyph_atlas = glyph_atlas_res.0.lock().unwrap();
            
            let (device, queue, command_pool, allocator) = {
                let vk_guard = vk_context_res.0.lock().unwrap();
                (
                    vk_guard.device.as_ref().unwrap().clone(),
                    vk_guard.queue.unwrap(),
                    vk_guard.command_pool.unwrap(),
                    vk_guard.allocator.as_ref().unwrap().clone()
                )
            };

            // We need to use with_buffer to get an immutable reference to the buffer for layout iteration.
            editor.with_buffer(|b| {
                for run in b.layout_runs() {
                    let baseline_y = -run.line_y;
                    for layout_glyph in run.glyphs.iter() {
                        let (cache_key, _, _) = CacheKey::new(layout_glyph.font_id, layout_glyph.glyph_id, layout_glyph.font_size, (layout_glyph.x, layout_glyph.y), CacheKeyFlags::empty());
                        let Some(swash_image) = swash_cache.get_image(&mut font_system.font_system, cache_key) else { continue; };
                        
                        if let Ok(glyph_info) = glyph_atlas.add_glyph(&device, queue, command_pool, &allocator, cache_key, &swash_image) {
                            let placement = swash_image.placement;
                            let width = placement.width as f32;
                            let height = placement.height as f32;

                            let relative_left_x = layout_glyph.x;
                            let relative_right_x = relative_left_x + width;
                            let relative_top_y = baseline_y + placement.top as f32;
                            let relative_bottom_y = relative_top_y - height;

                            let top_left = Vec2::new(relative_left_x, relative_top_y);
                            let top_right = Vec2::new(relative_right_x, relative_top_y);
                            let bottom_right = Vec2::new(relative_right_x, relative_bottom_y);
                            let bottom_left = Vec2::new(relative_left_x, relative_bottom_y);

                            positioned_glyphs.push(PositionedGlyph {
                                glyph_info: *glyph_info,
                                layout_glyph: layout_glyph.clone(),
                                vertices: [top_left, top_right, bottom_right, bottom_left],
                            });
                        }
                    }
                }
            });

            commands.entity(entity).insert(TextLayoutOutput {
                glyphs: positioned_glyphs,
            });

            if let Some(yrs_text) = yrs_doc_res.text_map.lock().unwrap().get(&entity) {
                let mut txn = yrs_doc_res.doc.transact_mut();
                if initial_cursor_pos > 0 {
                    yrs_text.remove_range(&mut txn, (initial_cursor_pos - 1) as u32, 1);
                }
            }

            yrs_text_changed_writer.send(YrsTextChanged { entity });

        } else if keyboard_input.just_pressed(KeyCode::Delete) {
            warn!("'Delete' key not yet implemented with local-first pattern.");
        } else {
            warn!("Character insertion not yet implemented with local-first pattern.");
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
                    if let Some(origin_x) = cursor_state.x_goal {
                        let target_line_index = if motion == Motion::Up {
                            cursor_state.line.saturating_sub(1)
                        } else {
                            cursor_state.line + 1
                        };

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

                    // This is the typo you found! Corrected `run` to `new_run`.
                    let new_run = original_buffer.layout_runs().find(|r| r.line_i == new_cursor.line).unwrap();
                    let glyph_at_cursor = new_run.glyphs.iter().find(|g| g.start == new_cursor.index);
                    let new_x = glyph_at_cursor.map_or_else(
                        || new_run.glyphs.first().map_or(0.0, |g| g.x) + new_run.line_w,
                        |g| g.x,
                    );
                    cursor_state.x_goal = Some(new_x as i32);

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