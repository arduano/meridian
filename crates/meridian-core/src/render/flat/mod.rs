use crate::midi::{MIDI_KEY_COUNT, views::MIDIFileViewsUnion};

use super::{
    SceneLayout, SceneProjector,
    shared::{ProjectedScene, SceneLayer, is_black_key, mix, solid_quad},
};

pub(crate) struct FlatProjector;

#[derive(Clone, Copy)]
struct FlatKeyState {
    x1: f32,
    x2: f32,
    is_black: bool,
    color: Option<[f32; 4]>,
}

fn zenith_white_key_bounds(mut x1: f32, mut x2: f32, key: usize) -> (f32, f32) {
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

impl SceneProjector for FlatProjector {
    fn project_into(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = false;
        let first_key = layout.first_key.min(layout.last_key) as usize;
        let last_key = layout.last_key.max(layout.first_key) as usize;
        let view_range = views.range().length() as f32;
        let piano_height = layout.piano_height;
        let key_width = 1.0 / ((last_key - first_key + 1) as f32);
        let mut key_states = Vec::with_capacity(last_key.saturating_sub(first_key) + 1);

        for key in first_key..=last_key.min(MIDI_KEY_COUNT - 1) {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let note_layer = if is_black {
                SceneLayer::BlackNotes
            } else {
                SceneLayer::WhiteNotes
            };
            let mut key_color = None;
            let column = views.get_column(key);

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
                    key_color = Some(color);
                }
                scene.visible_notes += 1;
                scene.note_quads += 1;
            });

            if key_color.is_some() {
                scene.active_keys += 1;
            }

            key_states.push(FlatKeyState {
                x1,
                x2,
                is_black,
                color: key_color,
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
            let color = key_state
                .color
                .map(|pressed| mix([1.0, 1.0, 1.0, 1.0], pressed, 0.96))
                .unwrap_or([1.0, 1.0, 1.0, 1.0]);

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
            let color = key_state
                .color
                .map(|pressed| mix(base, pressed, 0.96))
                .unwrap_or(base);
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
