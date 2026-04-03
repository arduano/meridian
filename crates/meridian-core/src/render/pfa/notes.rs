use rayon::prelude::*;

use crate::midi::{
    MIDI_KEY_COUNT, MIDINoteColumnView, MIDINoteViews, ram::view::InRamCurrentNoteViews,
    views::MIDIFileViewsUnion,
};

use super::model::{KeyColorPair, KeyNoteProjection, PfaLayoutParams, build_key_position_arrays};
use crate::render::{
    SceneLayout,
    shared::{
        KeyActivity, NoteInstance, NoteShaderKind, PfaNoteProjectorConfig, ProjectedScene,
        SceneLayer, alpha_blend, for_each_visible_note, is_black_key, normalized_key_range,
    },
};

pub(crate) fn project_pfa_notes(
    config: &PfaNoteProjectorConfig,
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    scene.set_note_shader_kind(NoteShaderKind::Pfa);
    scene.notes_black_first = true;
    let params = PfaLayoutParams::new(
        layout,
        piano_height,
        views.range().length() as f32,
        config.border_width,
    );
    let (first_note, last_note) = normalized_key_range(layout.first_key, layout.last_key);
    let arrays = build_key_position_arrays(first_note, last_note, config.same_width_notes);
    scene.set_note_params(
        params.piano_height,
        params.note_pos_factor,
        params.padding_x,
        params.padding_y,
    );
    for key in first_note..last_note {
        scene.set_note_key_x(
            key as u8,
            arrays.x1[key],
            arrays.x1[key] + arrays.width[key],
        );
    }

    match views {
        MIDIFileViewsUnion::InRam(views) => {
            project_note_pass_in_ram(scene, views, &params, first_note, last_note, false);
            project_note_pass_in_ram(scene, views, &params, first_note, last_note, true);
        }
    }
}

fn project_note_pass_in_ram(
    scene: &mut ProjectedScene,
    views: &InRamCurrentNoteViews<'_>,
    params: &PfaLayoutParams,
    first_note: usize,
    last_note: usize,
    only_black: bool,
) {
    let layer = if only_black {
        SceneLayer::BlackNotes
    } else {
        SceneLayer::WhiteNotes
    };
    let key_end = last_note.min(MIDI_KEY_COUNT);
    let results: Vec<KeyNoteProjection> = (first_note..key_end)
        .into_par_iter()
        .filter(|&key| only_black == is_black_key(key as u8))
        .map(|key| {
            let is_black = is_black_key(key as u8);
            let column = views.get_column(key);
            let mut notes = Vec::with_capacity(column.iterate_displaced_notes().len());
            let mut projected_key_color = KeyColorPair::default();
            let mut projected_key_pressed = false;

            for_each_visible_note(column, 0.0, params.view_range, false, false, |note| {
                let packed_left = note.color.left.to_rgba_packed(255);
                let packed_right = note.color.right.to_rgba_packed(255);
                if note.active {
                    let pair = KeyColorPair {
                        left: note.color.left.to_rgba(1.0),
                        right: note.color.right.to_rgba(1.0),
                    };
                    if is_black {
                        if !projected_key_pressed {
                            projected_key_color = pair;
                        }
                    } else {
                        projected_key_color.left = alpha_blend(projected_key_color.left, pair.left);
                        projected_key_color.right =
                            alpha_blend(projected_key_color.right, pair.right);
                    }
                    projected_key_pressed = true;
                }

                notes.push(NoteInstance::new(
                    key as u32,
                    note.start,
                    note.end,
                    packed_left,
                    packed_right,
                ));
            });

            KeyNoteProjection {
                key,
                notes,
                key_color: projected_key_color,
                key_pressed: projected_key_pressed,
            }
        })
        .collect();

    for result in results {
        scene.visible_notes += result.notes.len();
        scene.note_quads += result.notes.len();
        scene.set_key_activity(
            result.key,
            KeyActivity {
                pressed: result.key_pressed,
                left: result.key_color.left,
                right: result.key_color.right,
            },
        );
        if result.key_pressed {
            scene.active_keys += 1;
        }
        scene.extend_note_layer(layer, result.notes);
    }
}
