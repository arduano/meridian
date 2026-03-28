use crate::midi::MIDI_KEY_COUNT;

use super::super::shared::KeyActivity;

#[derive(Clone)]
pub(crate) struct FlatNoteProjector(pub super::super::shared::FlatNoteProjectorConfig);

#[derive(Clone, Copy)]
pub(crate) struct FlatKeyboardProjector(pub super::super::shared::FlatKeyboardProjectorConfig);

#[derive(Clone, Copy)]
pub(crate) struct FlatKeyState {
    pub x1: f32,
    pub x2: f32,
    pub is_black: bool,
    pub activity: KeyActivity,
}

pub(crate) fn key_span(layout: &super::super::SceneLayout) -> (usize, usize, f32) {
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
