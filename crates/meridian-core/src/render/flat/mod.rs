use crate::midi::{MIDI_KEY_COUNT, views::MIDIFileViewsUnion};

use super::{
    KeyboardProjector, NoteProjector, SceneLayout,
    shared::{
        FlatKeyboardProjectorConfig, FlatNoteProjectorConfig, KeyActivity, ProjectedScene,
        SceneLayer, is_black_key, mix, shade, solid_quad,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct FlatNoteProjector(pub FlatNoteProjectorConfig);

#[derive(Clone, Copy)]
pub(crate) struct FlatKeyboardProjector(pub FlatKeyboardProjectorConfig);

#[derive(Clone, Copy)]
struct FlatKeyState {
    x1: f32,
    x2: f32,
    is_black: bool,
    activity: KeyActivity,
}

fn key_span(layout: &SceneLayout) -> (usize, usize, f32) {
    let first_key = layout.first_key.min(layout.last_key) as usize;
    let last_key = layout.last_key.max(layout.first_key) as usize;
    let width = 1.0 / ((last_key - first_key + 1) as f32);
    (first_key, last_key.min(MIDI_KEY_COUNT - 1), width)
}

pub(crate) fn zenith_white_key_bounds(mut x1: f32, mut x2: f32, key: usize) -> (f32, f32) {
    let width = x2 - x1;
    match key % 12 {
        0 => x2 += width * 0.666,
        2 => {
            x1 -= width / 3.0;
            x2 += width / 3.0;
        }
        4 => x1 -= width * (2.0 / 3.0),
        5 => x2 += width * 0.75,
        7 => {
            x1 -= width * 0.25;
            x2 += width * 0.5;
        }
        9 => {
            x1 -= width * 0.5;
            x2 += width * 0.25;
        }
        11 => x1 -= width * 0.75,
        _ => {}
    }
    (x1, x2)
}

impl NoteProjector for FlatNoteProjector {
    fn project_notes(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    ) {
        let _ = self.0;
        scene.notes_black_first = false;
        let (first_key, last_key, key_width) = key_span(layout);
        let view_range = views.range().length() as f32;

        for key in first_key..=last_key {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let note_layer = if is_black {
                SceneLayer::BlackNotes
            } else {
                SceneLayer::WhiteNotes
            };
            let column = views.get_column(key);
            let mut activity = KeyActivity::default();

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= view_range {
                    return;
                }

                let bottom =
                    piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
                let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
                let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });

                scene.push_quad(note_layer, solid_quad(x1, bottom, x2, top, color));

                if note.start <= 0.0 && end > 0.0 {
                    activity.pressed = true;
                    activity.left = color;
                    activity.right = color;
                }
                scene.visible_notes += 1;
                scene.note_quads += 1;
            });

            if activity.pressed {
                scene.active_keys += 1;
            }
            scene.set_key_activity(key, activity);
        }
    }
}

impl KeyboardProjector for FlatKeyboardProjector {
    fn project_keyboard(
        &self,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    ) {
        let _ = self.0;
        let (first_key, last_key, key_width) = key_span(layout);
        let mut key_states = Vec::with_capacity(last_key.saturating_sub(first_key) + 1);

        for key in first_key..=last_key {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            key_states.push(FlatKeyState {
                x1,
                x2,
                is_black: is_black_key(key as u8),
                activity: scene.key_activity(key),
            });
        }

        scene.push_quad(
            SceneLayer::WhiteKeys,
            solid_quad(0.0, 0.0, 1.0, piano_height, [0.96, 0.96, 0.96, 1.0]),
        );
        scene.keyboard_quads += 1;

        for (offset, key_state) in key_states.iter().enumerate() {
            if key_state.is_black {
                continue;
            }

            let key = first_key + offset;
            let (x1, x2) = zenith_white_key_bounds(key_state.x1, key_state.x2, key);
            let pressed = key_state.activity.left;
            let color = if key_state.activity.pressed {
                mix([1.0, 1.0, 1.0, 1.0], pressed, 0.96)
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };

            scene.push_quad(
                SceneLayer::WhiteKeys,
                solid_quad(x1.max(0.0), 0.0, x2.min(1.0), piano_height, color),
            );
            scene.keyboard_quads += 1;
        }

        for key_state in &key_states {
            if !key_state.is_black {
                continue;
            }

            let base = [0.02, 0.02, 0.02, 1.0];
            let color = if key_state.activity.pressed {
                mix(base, key_state.activity.left, 0.96)
            } else {
                base
            };
            scene.push_quad(
                SceneLayer::BlackKeys,
                solid_quad(
                    key_state.x1,
                    piano_height * 0.37,
                    key_state.x2,
                    piano_height,
                    color,
                ),
            );
            scene.keyboard_quads += 1;
        }
    }
}

pub(crate) fn project_basic_notes(
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    scene.notes_black_first = false;
    let (first_key, last_key, key_width) = key_span(layout);
    let view_range = views.range().length() as f32;

    for key in first_key..=last_key {
        let x1 = (key - first_key) as f32 * key_width;
        let x2 = x1 + key_width;
        let is_black = is_black_key(key as u8);
        let mut activity = KeyActivity::default();
        let column = views.get_column(key);

        column.for_each_displaced_note(|note| {
            let end = note.start + note.len;
            if end <= 0.0 || note.start >= view_range {
                return;
            }

            let bottom = piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
            let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
            let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });
            let border = shade(color, if is_black { 0.18 } else { -0.18 });
            let inset = key_width * if is_black { 0.10 } else { 0.06 };
            let layer = if is_black {
                SceneLayer::BlackNotes
            } else {
                SceneLayer::WhiteNotes
            };

            scene.push_quad(layer, solid_quad(x1, bottom, x2, top, border));
            scene.push_quad(
                layer,
                solid_quad(
                    x1 + inset,
                    bottom + 0.003,
                    x2 - inset,
                    (top - 0.003).max(bottom + 0.003),
                    color,
                ),
            );

            if note.start <= 0.0 && end > 0.0 {
                activity.pressed = true;
                activity.left = color;
                activity.right = color;
            }
            scene.visible_notes += 1;
            scene.note_quads += 2;
        });

        if activity.pressed {
            scene.active_keys += 1;
        }
        scene.set_key_activity(key, activity);
    }
}

pub(crate) fn project_basic_keyboard(
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    let (first_key, last_key, key_width) = key_span(layout);
    for key in first_key..=last_key {
        let x1 = (key - first_key) as f32 * key_width;
        let x2 = x1 + key_width;
        let is_black = is_black_key(key as u8);
        let activity = scene.key_activity(key);
        let key_top = if is_black { piano_height * 0.62 } else { piano_height };
        let base = if is_black {
            [0.08, 0.16, 0.09, 1.0]
        } else {
            [0.74, 0.96, 0.76, 1.0]
        };
        let fill = if activity.pressed {
            mix(base, activity.left, if is_black { 0.75 } else { 0.45 })
        } else {
            base
        };
        let border = if is_black {
            shade(fill, 0.12)
        } else {
            shade(fill, -0.18)
        };
        let layer = if is_black {
            SceneLayer::BlackKeys
        } else {
            SceneLayer::WhiteKeys
        };

        scene.push_quad(layer, solid_quad(x1, 0.0, x2, key_top, border));
        scene.push_quad(
            layer,
            solid_quad(
                x1 + key_width * if is_black { 0.08 } else { 0.03 },
                piano_height * 0.012,
                x2 - key_width * if is_black { 0.08 } else { 0.03 },
                key_top - piano_height * 0.012,
                fill,
            ),
        );
        scene.keyboard_quads += 2;
    }
}
