use crate::midi::MIDI_KEY_COUNT;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BlackKeyProfile {
    pub width_factor: f32,
    pub offset_factors: [f32; 5],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyXLayout {
    pub x1: [f32; MIDI_KEY_COUNT + 1],
    pub width: [f32; MIDI_KEY_COUNT + 1],
    pub primary_width: f32,
}

pub(crate) const PFA_BLACK_KEY_PROFILE: BlackKeyProfile = BlackKeyProfile {
    width_factor: 0.64,
    offset_factors: [0.7, 0.3, 0.7, 0.5, 0.3],
};

pub(crate) const PIANO_TRAIL_CLASSIC_BLACK_KEY_PROFILE: BlackKeyProfile = BlackKeyProfile {
    width_factor: 0.6,
    offset_factors: [0.7, 1.0 / 2.8, 0.75, 0.5, 1.0 / 3.0],
};

pub(crate) const WHITE_KEY_EXPANSION: [(f32, f32); 12] = [
    (0.0, 0.666),
    (0.0, 0.0),
    (-1.0 / 3.0, 1.0 / 3.0),
    (0.0, 0.0),
    (-2.0 / 3.0, 0.0),
    (0.0, 0.75),
    (0.0, 0.0),
    (-0.25, 0.5),
    (0.0, 0.0),
    (-0.5, 0.25),
    (0.0, 0.0),
    (-0.75, 0.0),
];

const KEY_NUMBERS: [usize; MIDI_KEY_COUNT + 1] = build_key_numbers();

pub(crate) fn normalized_key_range(first_key: u8, last_key: u8) -> (usize, usize) {
    let first_key = first_key.min(last_key) as usize;
    let last_key_exclusive = (last_key.max(first_key as u8) as usize + 1).min(MIDI_KEY_COUNT);
    (first_key, last_key_exclusive.max(first_key + 1))
}

pub(crate) fn linear_key_span(first_key: usize, last_key_exclusive: usize) -> (usize, usize, f32) {
    let width = 1.0 / (last_key_exclusive.saturating_sub(first_key)).max(1) as f32;
    (first_key, last_key_exclusive.saturating_sub(1), width)
}

pub(crate) fn build_key_x_layout(
    first_key: usize,
    last_key_exclusive: usize,
    same_width: bool,
    black_keys: BlackKeyProfile,
) -> KeyXLayout {
    let last_key_exclusive = last_key_exclusive.min(MIDI_KEY_COUNT).max(first_key + 1);
    let mut layout = KeyXLayout {
        x1: [0.0; MIDI_KEY_COUNT + 1],
        width: [0.0; MIDI_KEY_COUNT + 1],
        primary_width: 0.0,
    };

    if same_width {
        let denom = (last_key_exclusive - first_key).max(1) as f32;
        for key in 0..=MIDI_KEY_COUNT {
            layout.x1[key] = (key.saturating_sub(first_key)) as f32 / denom;
            layout.width[key] = 1.0 / denom;
        }
        layout.primary_width = 1.0 / denom;
        return layout;
    }

    let mut left = KEY_NUMBERS[first_key] as f32;
    let mut right = KEY_NUMBERS[last_key_exclusive - 1] as f32;
    if first_key > 0 && is_black_key(first_key as u8) {
        left = KEY_NUMBERS[first_key - 1] as f32 + 0.5;
    }
    if last_key_exclusive < MIDI_KEY_COUNT && is_black_key((last_key_exclusive - 1) as u8) {
        right = KEY_NUMBERS[last_key_exclusive] as f32 - 0.5;
    }

    let denom = (right - left + 1.0).max(1.0);
    for (key, key_number) in KEY_NUMBERS.iter().enumerate().take(MIDI_KEY_COUNT + 1) {
        if !is_black_key(key as u8) {
            layout.x1[key] = (*key_number as f32 - left) / denom;
            layout.width[key] = 1.0 / denom;
        } else {
            let width = black_keys.width_factor / denom;
            let offset = width * black_keys.offset_factors[*key_number % 5];
            let next = (key + 1).min(MIDI_KEY_COUNT);
            layout.x1[key] = (KEY_NUMBERS[next] as f32 - left) / denom - offset;
            layout.width[key] = width;
        }
    }
    layout.primary_width = 1.0 / denom;
    layout
}

pub(crate) fn expanded_white_key_bounds(x1: f32, x2: f32, key: usize) -> (f32, f32) {
    let width = x2 - x1;
    let (left, right) = WHITE_KEY_EXPANSION[key % 12];
    (x1 + width * left, x2 + width * right)
}

const fn build_key_numbers() -> [usize; MIDI_KEY_COUNT + 1] {
    let mut keynum = [0; MIDI_KEY_COUNT + 1];
    let mut black = 0usize;
    let mut white = 0usize;
    let mut key = 0usize;
    while key <= MIDI_KEY_COUNT {
        if is_black_key(key as u8) {
            keynum[key] = black;
            black += 1;
        } else {
            keynum[key] = white;
            white += 1;
        }
        key += 1;
    }
    keynum
}

pub(crate) const fn is_black_key(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}
