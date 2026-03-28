use crate::midi::MIDI_KEY_COUNT;

use crate::render::shared::{MiditrailSceneConfig, is_black_key};

#[derive(Clone, Copy, Debug)]
pub struct MiditrailLayout {
    pub first_key: usize,
    pub last_key_exclusive: usize,
    pub x1: [f32; MIDI_KEY_COUNT],
    pub width: [f32; MIDI_KEY_COUNT],
    pub circle_radius: f32,
}

impl MiditrailLayout {
    pub fn new(first_key: usize, last_key_exclusive: usize, config: &MiditrailSceneConfig) -> Self {
        let last_key_exclusive = last_key_exclusive.min(MIDI_KEY_COUNT);
        let mut x1 = [0.0; MIDI_KEY_COUNT];
        let mut width = [0.0; MIDI_KEY_COUNT];
        let mut keynum = [0usize; MIDI_KEY_COUNT + 1];
        let mut black_count = 0usize;
        let mut white_count = 0usize;
        for (key, slot) in keynum.iter_mut().enumerate() {
            if is_black_key(key as u8) {
                *slot = black_count;
                black_count += 1;
            } else {
                *slot = white_count;
                white_count += 1;
            }
        }

        let circle_radius = if config.same_width_notes {
            let denom = (last_key_exclusive.saturating_sub(first_key)).max(1) as f32;
            for key in 0..=MIDI_KEY_COUNT.min(last_key_exclusive) {
                x1[key] = (key.saturating_sub(first_key)) as f32 / denom;
                width[key] = 1.0 / denom;
            }
            1.0 / denom
        } else {
            let mut knmfn = keynum[first_key] as f32;
            let mut knmln = keynum[last_key_exclusive.saturating_sub(1)] as f32;
            if first_key > 0 && is_black_key(first_key as u8) {
                knmfn = keynum[first_key - 1] as f32 + 0.5;
            }
            if last_key_exclusive < MIDI_KEY_COUNT && is_black_key((last_key_exclusive - 1) as u8) {
                knmln = keynum[last_key_exclusive] as f32 - 0.5;
            }
            let denom = (knmln - knmfn + 1.0).max(1.0);
            for key in 0..MIDI_KEY_COUNT.min(last_key_exclusive + 1) {
                if !is_black_key(key as u8) {
                    x1[key] = (keynum[key] as f32 - knmfn) / denom;
                    width[key] = 1.0 / denom;
                } else {
                    let w = 0.6 / denom;
                    let bknum = keynum[key] % 5;
                    let mut offset = w / 2.0;
                    if bknum == 0 {
                        offset *= 1.4;
                    } else if bknum == 1 {
                        offset /= 1.4;
                    }
                    if bknum == 2 {
                        offset *= 1.5;
                    } else if bknum == 4 {
                        offset /= 1.5;
                    }
                    x1[key] = (keynum[key + 1] as f32 - knmfn) / denom - offset;
                    width[key] = w;
                }
            }
            0.6 / denom
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
        let mut x1 = self.x1[key];
        let mut x2 = x1 + self.width[key];
        if !is_black_key(key as u8) && self.first_key < self.last_key_exclusive {
            match key % 12 {
                0 => x2 += self.width[key] * 0.666,
                2 => {
                    x1 -= self.width[key] / 3.0;
                    x2 += self.width[key] / 3.0;
                }
                4 => x1 -= self.width[key] * (2.0 / 3.0),
                5 => x2 += self.width[key] * 0.75,
                7 => {
                    x1 -= self.width[key] / 4.0;
                    x2 += self.width[key] / 2.0;
                }
                9 => {
                    x1 -= self.width[key] / 2.0;
                    x2 += self.width[key] / 4.0;
                }
                11 => x1 -= self.width[key] * 0.75,
                _ => {}
            }
        }
        (x1 - 0.5, x2 - 0.5)
    }
}
