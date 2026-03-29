use crate::{
    midi::MIDI_KEY_COUNT, midi::views::MIDIFileViewsUnion, render::shared::MiditrailSceneConfig,
};

#[derive(Clone, Debug)]
pub struct MiditrailPhysicsState {
    pub key_press: [f32; MIDI_KEY_COUNT],
}

impl Default for MiditrailPhysicsState {
    fn default() -> Self {
        Self {
            key_press: [0.0; MIDI_KEY_COUNT],
        }
    }
}

pub fn tick_miditrail_physics(
    state: &mut MiditrailPhysicsState,
    config: &MiditrailSceneConfig,
    views: &MIDIFileViewsUnion<'_>,
    delta_seconds: f32,
) {
    let steps = (delta_seconds * 60.0).max(0.0);
    if steps <= 0.0 {
        return;
    }

    let decay_divisor = 1.05_f32.powf(steps);
    let decay_subtract = config.note_up_speed.max(0.0) * steps;
    for press in &mut state.key_press {
        *press = (*press / decay_divisor - decay_subtract).max(0.0);
    }

    let press_add = config.note_down_speed.max(0.0) * steps;
    if press_add <= 0.0 {
        return;
    }

    for key in 0..MIDI_KEY_COUNT {
        let column = views.get_column(key);
        column.for_each_displaced_note(|note| {
            let end = note.start + note.len;
            if note.start <= 0.0 && end > 0.0 {
                state.key_press[key] = (state.key_press[key] + press_add).min(1.0);
            }
        });
    }
}
