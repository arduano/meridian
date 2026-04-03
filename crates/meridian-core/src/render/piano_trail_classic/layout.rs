use crate::midi::MIDI_KEY_COUNT;

use crate::render::shared::{
    PIANO_TRAIL_CLASSIC_BLACK_KEY_PROFILE, PianoTrailClassicSceneConfig, build_key_x_layout,
    expanded_white_key_bounds,
};

#[derive(Clone, Copy, Debug)]
pub struct PianoTrailClassicLayout {
    pub first_key: usize,
    pub last_key_exclusive: usize,
    pub x1: [f32; MIDI_KEY_COUNT],
    pub width: [f32; MIDI_KEY_COUNT],
    pub circle_radius: f32,
}

impl PianoTrailClassicLayout {
    pub fn new(
        first_key: usize,
        last_key_exclusive: usize,
        config: &PianoTrailClassicSceneConfig,
    ) -> Self {
        let last_key_exclusive = last_key_exclusive.min(MIDI_KEY_COUNT);
        let shared = build_key_x_layout(
            first_key,
            last_key_exclusive,
            config.same_width_notes,
            PIANO_TRAIL_CLASSIC_BLACK_KEY_PROFILE,
        );
        let x1 = std::array::from_fn(|key| shared.x1[key]);
        let width = std::array::from_fn(|key| shared.width[key]);
        let circle_radius = if config.same_width_notes {
            shared.primary_width
        } else {
            PIANO_TRAIL_CLASSIC_BLACK_KEY_PROFILE.width_factor * shared.primary_width
        };

        Self {
            first_key,
            last_key_exclusive,
            x1,
            width,
            circle_radius,
        }
    }

    pub fn key_x1(&self, key: usize) -> f32 {
        self.x1[key] - 0.5
    }

    pub fn key_width(&self, key: usize) -> f32 {
        self.width[key]
    }

    pub fn expanded_white_key_span(&self, key: usize) -> (f32, f32) {
        let (x1, x2) = expanded_white_key_bounds(self.x1[key], self.x1[key] + self.width[key], key);
        (x1 - 0.5, x2 - 0.5)
    }
}
