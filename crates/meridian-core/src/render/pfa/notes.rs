use rayon::prelude::*;

use crate::midi::{
    MIDI_KEY_COUNT, MIDINoteColumnView, MIDINoteViews, ram::view::InRamCurrentNoteViews,
    views::MIDIFileViewsUnion,
};

use super::model::{
    KeyColorPair, KeyNoteProjection, KeyPositionArrays, NoteInstance, PfaLayoutParams,
    PfaNoteProjector,
};
use crate::render::{
    NoteProjector, SceneLayout,
    shared::{KeyActivity, ProjectedScene, SceneLayer, alpha_blend, is_black_key},
};

impl NoteProjector for PfaNoteProjector {
    fn project_notes(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = self.0.black_notes_above;
        let params = PfaLayoutParams::new(
            layout,
            piano_height,
            views.range().length() as f32,
            self.0.border_width,
        );
        let first_note = layout.first_key.min(layout.last_key) as usize;
        let last_note = layout.last_key.max(layout.first_key) as usize + 1;
        let arrays = KeyPositionArrays::new(first_note, last_note, self.0.same_width_notes);

        if self.0.black_notes_above {
            self.project_note_pass(scene, views, &params, &arrays, first_note, last_note, false);
            self.project_note_pass(scene, views, &params, &arrays, first_note, last_note, true);
        } else {
            self.project_note_pass(scene, views, &params, &arrays, first_note, last_note, false);
        }
    }
}

impl PfaNoteProjector {
    fn project_note_pass(
        &self,
        scene: &mut ProjectedScene,
        views: &MIDIFileViewsUnion<'_>,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        first_note: usize,
        last_note: usize,
        only_black: bool,
    ) {
        let layer = if only_black {
            SceneLayer::BlackNotes
        } else {
            SceneLayer::WhiteNotes
        };
        scene.set_note_params(
            params.piano_height,
            params.note_pos_factor,
            params.padding_x,
            params.padding_y,
        );
        match views {
            MIDIFileViewsUnion::InRam(views) => {
                self.project_note_pass_in_ram(
                    scene, views, params, arrays, first_note, last_note, only_black, layer,
                );
            }
        }
    }

    fn project_note_pass_in_ram(
        &self,
        scene: &mut ProjectedScene,
        views: &InRamCurrentNoteViews<'_>,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        first_note: usize,
        last_note: usize,
        only_black: bool,
        layer: SceneLayer,
    ) {
        let key_end = last_note.min(MIDI_KEY_COUNT);
        let results: Vec<KeyNoteProjection> = (first_note..key_end)
            .into_par_iter()
            .filter(|&key| only_black == is_black_key(key as u8))
            .map(|key| {
                let is_black = is_black_key(key as u8);
                let column = views.get_column(key);
                let iter = column.iterate_displaced_notes();
                let mut notes = Vec::with_capacity(iter.len());
                let mut projected_key_color = KeyColorPair::default();
                let mut projected_key_pressed = false;

                for note in iter {
                    let end = note.start + note.len;
                    if end <= 0.0 || note.start >= params.view_range {
                        continue;
                    }

                    let packed_color = note.color.to_rgba_packed(255);
                    if note.start <= 0.0 && end > 0.0 {
                        let rgba = note.color.to_rgba(1.0);
                        if is_black && self.0.black_notes_above {
                            projected_key_color = KeyColorPair {
                                left: rgba,
                                right: rgba,
                            };
                        } else {
                            projected_key_color.left = alpha_blend(rgba, projected_key_color.left);
                            projected_key_color.right =
                                alpha_blend(rgba, projected_key_color.right);
                        }
                        projected_key_pressed = true;
                    }

                    notes.push(NoteInstance {
                        key: key as u32,
                        start: note.start.max(0.0),
                        end: end.min(params.view_range),
                        color: packed_color,
                        _padding: [0; 3],
                    });
                }

                KeyNoteProjection {
                    key,
                    notes,
                    key_color: projected_key_color,
                    key_pressed: projected_key_pressed,
                }
            })
            .collect();

        for result in results {
            scene.set_note_key_x(
                result.key as u8,
                arrays.x1[result.key],
                arrays.x1[result.key] + arrays.width[result.key],
            );
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
}
