mod black_keys;
mod decorations;
mod white_keys;

use crate::{
    midi::MIDI_KEY_COUNT,
    render::{
        KeyboardProjector, SceneLayout,
        shared::{ProjectedScene, is_black_key},
    },
};

use super::model::{KeyPositionArrays, PfaKeyboardProjector, PfaLayoutParams};

impl KeyboardProjector for PfaKeyboardProjector {
    fn project_keyboard(
        &self,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    ) {
        let params = PfaLayoutParams::new(layout, piano_height, layout.view_range as f32, 1.0);
        let first_note = layout.first_key.min(layout.last_key) as usize;
        let last_note = layout.last_key.max(layout.first_key) as usize + 1;
        let arrays = KeyPositionArrays::new(first_note, last_note, self.0.same_width_notes);
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

        decorations::push_keyboard_decorations(self, scene, &params);
        white_keys::push_white_keys(self, scene, &params, &arrays, kbfirst, kblast);
        black_keys::push_black_keys(scene, &params, &arrays, kbfirst, kblast);
    }
}
