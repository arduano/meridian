use crate::render::{
    SceneLayout,
    shared::{KeyActivity, KeyXLayout, PFA_BLACK_KEY_PROFILE, alpha_blend, build_key_x_layout},
};

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

pub(crate) type KeyPositionArrays = KeyXLayout;

pub(crate) fn build_key_position_arrays(
    first_note: usize,
    last_note: usize,
    same_width: bool,
) -> KeyPositionArrays {
    build_key_x_layout(first_note, last_note, same_width, PFA_BLACK_KEY_PROFILE)
}

pub(crate) struct KeyNoteProjection {
    pub key: usize,
    pub notes: Vec<crate::render::shared::NoteInstance>,
    pub key_color: KeyColorPair,
    pub key_pressed: bool,
}

pub(crate) fn blend_key_pair(pair: KeyColorPair, base: KeyColorPair) -> KeyColorPair {
    KeyColorPair {
        left: alpha_blend(pair.left, base.left),
        right: alpha_blend(pair.right, base.right),
    }
}

pub(crate) fn activity_pair(activity: KeyActivity) -> KeyColorPair {
    KeyColorPair {
        left: activity.left,
        right: activity.right,
    }
}
