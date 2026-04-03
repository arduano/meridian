mod black_keys;
mod decorations;
mod white_keys;

use crate::{
    midi::MIDI_KEY_COUNT,
    render::{
        SceneLayout,
        shared::{PfaKeyboardProjectorConfig, ProjectedScene, is_black_key, normalized_key_range},
    },
};

use super::model::{PfaLayoutParams, build_key_position_arrays};

pub(crate) fn project_pfa_keyboard(
    config: &PfaKeyboardProjectorConfig,
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    let params = PfaLayoutParams::new(layout, piano_height, layout.view_range as f32, 1.0);
    let (first_note, last_note) = normalized_key_range(layout.first_key, layout.last_key);
    let arrays = build_key_position_arrays(first_note, last_note, config.same_width_notes);
    let kbfirst = if is_black_key(first_note as u8) && first_note > 0 {
        first_note - 1
    } else {
        first_note
    };
    let mut kblast = last_note;
    if last_note < MIDI_KEY_COUNT && is_black_key((last_note - 1) as u8) {
        kblast += 1;
    }
    kblast = kblast.min(MIDI_KEY_COUNT);

    decorations::push_keyboard_decorations(config, scene, &params);
    white_keys::push_white_keys(config, scene, &params, &arrays, kbfirst, kblast);
    black_keys::push_black_keys(scene, &params, &arrays, kbfirst, kblast);
}
