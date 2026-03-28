use crate::{
    midi::MIDI_KEY_COUNT,
    render::{
        SceneLayout,
        shared::{
            KeyActivity, PfaKeyboardProjectorConfig, PfaNoteProjectorConfig, alpha_blend,
            is_black_key,
        },
    },
};

#[repr(C)]
#[derive(
    Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, serde::Serialize, serde::Deserialize,
)]
pub struct NoteInstance {
    pub key: u32,
    pub start: f32,
    pub end: f32,
    pub color: u32,
    pub _padding: [u32; 3],
}

#[derive(Clone, Copy)]
pub(crate) struct PfaNoteProjector(pub PfaNoteProjectorConfig);

#[derive(Clone, Copy)]
pub(crate) struct PfaKeyboardProjector(pub PfaKeyboardProjectorConfig);

#[derive(Clone, Copy, Default)]
pub(crate) struct KeyColorPair {
    pub left: [f32; 4],
    pub right: [f32; 4],
}

impl KeyColorPair {
    pub(crate) fn for_base(base: [f32; 4]) -> Self {
        Self {
            left: base,
            right: base,
        }
    }
}

pub(crate) struct PfaLayoutParams {
    pub piano_height: f32,
    pub view_range: f32,
    pub note_pos_factor: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub scwidth: f32,
    pub top_red_start: f32,
    pub top_red_end: f32,
    pub top_bar_end: f32,
    pub w_end_up_t: f32,
    pub w_end_up_b: f32,
    pub w_end_down_t: f32,
    pub b_key_end: f32,
    pub b_key_down_t: f32,
    pub b_key_down_b: f32,
    pub b_key_up_t: f32,
    pub b_key_up_b: f32,
    pub b_key_u_split_lt: f32,
    pub b_key_u_split_rt: f32,
    pub b_key_u_split_lb: f32,
    pub b_key_u_split_rb: f32,
}

impl PfaLayoutParams {
    pub(crate) fn new(
        layout: &SceneLayout,
        piano_height: f32,
        view_range: f32,
        border_width: f32,
    ) -> Self {
        let padding_x = 0.001 * border_width;
        let padding_y =
            padding_x * layout.viewport_width as f32 / layout.viewport_height.max(1) as f32;

        let mut params = Self {
            piano_height,
            view_range,
            note_pos_factor: (1.0 - piano_height) / view_range.max(0.001),
            padding_x,
            padding_y,
            scwidth: layout.viewport_width as f32,
            top_red_start: piano_height * 0.99,
            top_red_end: piano_height * 0.94,
            top_bar_end: piano_height * 0.927,
            w_end_up_t: piano_height * 0.05,
            w_end_up_b: piano_height * 0.035,
            w_end_down_t: piano_height * 0.01,
            b_key_end: piano_height * 0.345,
            b_key_down_t: 0.0,
            b_key_down_b: 0.0,
            b_key_up_t: 0.0,
            b_key_up_b: 0.0,
            b_key_u_split_lt: piano_height * 0.78,
            b_key_u_split_rt: piano_height * 0.71,
            b_key_u_split_lb: piano_height * 0.65,
            b_key_u_split_rb: piano_height * 0.58,
        };
        params.b_key_down_t = params.top_bar_end + piano_height * 0.015;
        params.b_key_down_b = params.b_key_end + piano_height * 0.015;
        params.b_key_up_t = params.top_bar_end + piano_height * 0.045;
        params.b_key_up_b = params.b_key_end + piano_height * 0.045;
        params
    }
}

pub(crate) struct KeyPositionArrays {
    pub x1: [f32; MIDI_KEY_COUNT + 1],
    pub width: [f32; MIDI_KEY_COUNT + 1],
}

impl KeyPositionArrays {
    pub(crate) fn new(first_note: usize, last_note: usize, same_width: bool) -> Self {
        let mut result = Self {
            x1: [0.0; MIDI_KEY_COUNT + 1],
            width: [0.0; MIDI_KEY_COUNT + 1],
        };
        let keynum = build_key_numbers();

        if same_width {
            for i in 0..=MIDI_KEY_COUNT {
                result.x1[i] =
                    (i.saturating_sub(first_note)) as f32 / (last_note - first_note) as f32;
                result.width[i] = 1.0 / (last_note - first_note) as f32;
            }
        } else {
            let mut knmfn = keynum[first_note] as f32;
            let mut knmln = keynum[last_note - 1] as f32;
            if is_black_key(first_note as u8) && first_note > 0 {
                knmfn = keynum[first_note - 1] as f32 + 0.5;
            }
            if is_black_key((last_note - 1) as u8) && last_note < MIDI_KEY_COUNT {
                knmln = keynum[last_note] as f32 - 0.5;
            }
            let norm = knmln - knmfn + 1.0;
            for i in 0..=MIDI_KEY_COUNT {
                if !is_black_key(i as u8) {
                    result.x1[i] = (keynum[i] as f32 - knmfn) / norm;
                    result.width[i] = 1.0 / norm;
                } else {
                    let width = 0.64 / norm;
                    let bknum = keynum[i] % 5;
                    let mut offset = width / 2.0;
                    if bknum == 0 || bknum == 2 {
                        offset += offset * 0.4;
                    }
                    if bknum == 1 || bknum == 4 {
                        offset -= offset * 0.4;
                    }
                    let next = (i + 1).min(MIDI_KEY_COUNT);
                    result.x1[i] = (keynum[next] as f32 - knmfn) / norm - offset;
                    result.width[i] = width;
                }
            }
        }

        result
    }
}

pub(crate) struct KeyNoteProjection {
    pub key: usize,
    pub notes: Vec<NoteInstance>,
    pub key_color: KeyColorPair,
    pub key_pressed: bool,
}

pub(crate) fn blend_key_pair(pair: KeyColorPair, base: KeyColorPair) -> KeyColorPair {
    KeyColorPair {
        left: alpha_blend(pair.left, base.left),
        right: alpha_blend(pair.right, base.right),
    }
}

fn build_key_numbers() -> [i32; MIDI_KEY_COUNT + 1] {
    let mut keynum = [0; MIDI_KEY_COUNT + 1];
    let mut b = 0;
    let mut w = 0;
    for (i, slot) in keynum.iter_mut().enumerate() {
        if is_black_key(i as u8) {
            *slot = b;
            b += 1;
        } else {
            *slot = w;
            w += 1;
        }
    }
    keynum
}

pub(crate) fn activity_pair(activity: KeyActivity) -> KeyColorPair {
    KeyColorPair {
        left: activity.left,
        right: activity.right,
    }
}
