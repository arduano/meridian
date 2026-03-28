pub mod wgpu;

use rayon::prelude::*;

use crate::midi::{
    MIDI_KEY_COUNT, MIDINoteColumnView, MIDINoteViews, ram::view::InRamCurrentNoteViews,
    views::MIDIFileViewsUnion,
};

use super::{
    SceneLayout, SceneProjector,
    shared::{ProjectedScene, SceneLayer, is_black_key, mod_add, quad, vertical_gradient_quad},
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
pub(super) struct PfaProjector {
    same_width_notes: bool,
    black_notes_above: bool,
    middle_c: bool,
    border_width: f32,
    top_color: PfaTopColor,
    top_bar_rgb: [f32; 3],
}

impl Default for PfaProjector {
    fn default() -> Self {
        Self {
            same_width_notes: false,
            black_notes_above: true,
            middle_c: false,
            border_width: 1.0,
            top_color: PfaTopColor::Red,
            top_bar_rgb: [0.585, 0.0392, 0.0249],
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum PfaTopColor {
    Red,
    Blue,
    Green,
}

#[derive(Clone, Copy, Default)]
struct KeyColorPair {
    left: [f32; 4],
    right: [f32; 4],
}

impl KeyColorPair {
    fn for_base(base: [f32; 4]) -> Self {
        Self {
            left: base,
            right: base,
        }
    }
}

impl SceneProjector for PfaProjector {
    fn project_into(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = self.black_notes_above;
        let params = PfaLayoutParams::new(layout, views.range().length() as f32, self.border_width);
        let first_note = layout.first_key.min(layout.last_key) as usize;
        let last_note = layout.last_key.max(layout.first_key) as usize + 1;
        let arrays = KeyPositionArrays::new(first_note, last_note, self.same_width_notes);

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

        let mut key_colors = [KeyColorPair::default(); MIDI_KEY_COUNT];
        let mut key_pressed = [false; MIDI_KEY_COUNT];
        let orig_white = KeyColorPair::for_base([1.0, 1.0, 1.0, 1.0]);
        let orig_black = KeyColorPair::for_base([0.0, 0.0, 0.0, 1.0]);

        self.push_keyboard_decorations(scene, &params);

        if self.black_notes_above {
            self.project_note_pass(
                scene,
                &views,
                &params,
                &arrays,
                first_note,
                last_note,
                false,
                &mut key_colors,
                &mut key_pressed,
            );
            self.project_note_pass(
                scene,
                &views,
                &params,
                &arrays,
                first_note,
                last_note,
                true,
                &mut key_colors,
                &mut key_pressed,
            );
        } else {
            self.project_note_pass(
                scene,
                &views,
                &params,
                &arrays,
                first_note,
                last_note,
                false,
                &mut key_colors,
                &mut key_pressed,
            );
        }

        self.push_white_keys(
            scene,
            &params,
            &arrays,
            kbfirst,
            kblast,
            &key_colors,
            &key_pressed,
            orig_white,
        );
        self.push_black_keys(
            scene,
            &params,
            &arrays,
            kbfirst,
            kblast,
            &key_colors,
            &key_pressed,
            orig_black,
        );
    }
}

struct PfaLayoutParams {
    piano_height: f32,
    view_range: f32,
    note_pos_factor: f32,
    padding_x: f32,
    padding_y: f32,
    scwidth: f32,
    top_red_start: f32,
    top_red_end: f32,
    top_bar_end: f32,
    w_end_up_t: f32,
    w_end_up_b: f32,
    w_end_down_t: f32,
    b_key_end: f32,
    b_key_down_t: f32,
    b_key_down_b: f32,
    b_key_up_t: f32,
    b_key_up_b: f32,
    b_key_u_split_lt: f32,
    b_key_u_split_rt: f32,
    b_key_u_split_lb: f32,
    b_key_u_split_rb: f32,
}

impl PfaLayoutParams {
    fn new(layout: &SceneLayout, view_range: f32, border_width: f32) -> Self {
        let piano_height = layout.piano_height;
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

struct KeyPositionArrays {
    x1: [f32; MIDI_KEY_COUNT + 1],
    width: [f32; MIDI_KEY_COUNT + 1],
}

impl KeyPositionArrays {
    fn new(first_note: usize, last_note: usize, same_width: bool) -> Self {
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

impl PfaProjector {
    #[allow(clippy::too_many_arguments)]
    fn project_note_pass(
        &self,
        scene: &mut ProjectedScene,
        views: &MIDIFileViewsUnion<'_>,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        first_note: usize,
        last_note: usize,
        only_black: bool,
        key_colors: &mut [KeyColorPair; MIDI_KEY_COUNT],
        key_pressed: &mut [bool; MIDI_KEY_COUNT],
    ) {
        let layer = if only_black {
            SceneLayer::BlackNotes
        } else {
            SceneLayer::WhiteNotes
        };
        scene.set_note_params(
            params.piano_height,
            params.note_pos_factor,
            params.padding_x,
            params.padding_y,
        );
        match views {
            MIDIFileViewsUnion::InRam(views) => {
                self.project_note_pass_in_ram(
                    scene,
                    views,
                    params,
                    arrays,
                    first_note,
                    last_note,
                    only_black,
                    key_colors,
                    key_pressed,
                    layer,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn project_note_pass_in_ram(
        &self,
        scene: &mut ProjectedScene,
        views: &InRamCurrentNoteViews<'_>,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        first_note: usize,
        last_note: usize,
        only_black: bool,
        key_colors: &mut [KeyColorPair; MIDI_KEY_COUNT],
        key_pressed: &mut [bool; MIDI_KEY_COUNT],
        layer: SceneLayer,
    ) {
        let key_end = last_note.min(MIDI_KEY_COUNT);
        let results: Vec<KeyNoteProjection> = (first_note..key_end)
            .into_par_iter()
            .filter(|&key| only_black == is_black_key(key as u8))
            .map(|key| {
                let is_black = is_black_key(key as u8);
                let column = views.get_column(key);
                let iter = column.iterate_displaced_notes();
                let mut notes = Vec::with_capacity(iter.len());
                let mut projected_key_color = KeyColorPair::default();
                let mut projected_key_pressed = false;

                for note in iter {
                    let end = note.start + note.len;
                    if end <= 0.0 || note.start >= params.view_range {
                        continue;
                    }

                    let packed_color = note.color.to_rgba_packed(255);
                    if note.start <= 0.0 && end > 0.0 {
                        let rgba = note.color.to_rgba(1.0);
                        if is_black && self.black_notes_above {
                            projected_key_color = KeyColorPair {
                                left: rgba,
                                right: rgba,
                            };
                        } else {
                            projected_key_color.left = alpha_blend(rgba, projected_key_color.left);
                            projected_key_color.right =
                                alpha_blend(rgba, projected_key_color.right);
                        }
                        projected_key_pressed = true;
                    }

                    notes.push(NoteInstance {
                        key: key as u32,
                        start: note.start.max(0.0),
                        end: end.min(params.view_range),
                        color: packed_color,
                        _padding: [0; 3],
                    });
                }

                KeyNoteProjection {
                    key,
                    notes,
                    key_color: projected_key_color,
                    key_pressed: projected_key_pressed,
                }
            })
            .collect();

        for result in results {
            scene.set_note_key_x(
                result.key as u8,
                arrays.x1[result.key],
                arrays.x1[result.key] + arrays.width[result.key],
            );
            scene.visible_notes += result.notes.len();
            scene.note_quads += result.notes.len();
            key_colors[result.key] = result.key_color;
            key_pressed[result.key] = result.key_pressed;
            scene.extend_note_layer(layer, result.notes);
        }
    }

    fn push_keyboard_decorations(&self, scene: &mut ProjectedScene, params: &PfaLayoutParams) {
        scene.push_quad(
            SceneLayer::KeyboardDecor,
            vertical_gradient_quad(
                0.0,
                params.top_red_start,
                1.0,
                params.piano_height,
                [0.0196, 0.0196, 0.0196, 1.0],
                [0.0196, 0.0196, 0.0196, 1.0],
                [0.086, 0.086, 0.086, 1.0],
                [0.086, 0.086, 0.086, 1.0],
            ),
        );

        let (top, bottom) = match self.top_color {
            PfaTopColor::Red => (
                [
                    self.top_bar_rgb[0] * 0.5,
                    self.top_bar_rgb[1] * 0.5,
                    self.top_bar_rgb[2] * 0.5,
                    1.0,
                ],
                [
                    self.top_bar_rgb[0],
                    self.top_bar_rgb[1],
                    self.top_bar_rgb[2],
                    1.0,
                ],
            ),
            PfaTopColor::Blue => ([0.0196, 0.0274, 0.313, 1.0], [0.0392, 0.0249, 0.585, 1.0]),
            PfaTopColor::Green => ([0.0274, 0.313, 0.0196, 1.0], [0.0249, 0.585, 0.0392, 1.0]),
        };
        scene.push_quad(
            SceneLayer::KeyboardDecor,
            vertical_gradient_quad(
                0.0,
                params.top_red_end,
                1.0,
                params.top_red_start,
                bottom,
                bottom,
                top,
                top,
            ),
        );
        scene.push_quad(
            SceneLayer::KeyboardDecor,
            vertical_gradient_quad(
                0.0,
                params.top_bar_end,
                1.0,
                params.top_red_end,
                [0.239, 0.239, 0.239, 1.0],
                [0.239, 0.239, 0.239, 1.0],
                [0.239, 0.239, 0.239, 1.0],
                [0.239, 0.239, 0.239, 1.0],
            ),
        );
        scene.keyboard_quads += 3;
    }

    fn push_white_keys(
        &self,
        scene: &mut ProjectedScene,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        kbfirst: usize,
        kblast: usize,
        key_colors: &[KeyColorPair; MIDI_KEY_COUNT],
        key_pressed: &[bool; MIDI_KEY_COUNT],
        orig_white: KeyColorPair,
    ) {
        for n in kbfirst..kblast {
            if is_black_key(n as u8) {
                continue;
            }
            let (mut x1, mut x2) = (arrays.x1[n], arrays.x1[n] + arrays.width[n]);
            let width = x2 - x1;

            if self.same_width_notes {
                match n % 12 {
                    0 => x2 += width * 0.666,
                    2 => {
                        x1 -= width / 3.0;
                        x2 += width / 3.0;
                    }
                    4 => x1 -= width * 0.666,
                    5 => x2 += width * 0.75,
                    7 => {
                        x1 -= width / 4.0;
                        x2 += width / 2.0;
                    }
                    9 => {
                        x1 -= width / 2.0;
                        x2 += width / 4.0;
                    }
                    11 => x1 -= width * 0.75,
                    _ => {}
                }
            }

            let pair = blend_key_pair(key_colors[n], orig_white);
            if key_pressed[n] {
                scene.active_keys += 1;
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        x1,
                        params.w_end_down_t,
                        x2,
                        params.top_bar_end,
                        pair.left,
                        pair.right,
                        mod_add(pair.right, 0.0, 0.5),
                        mod_add(pair.left, 0.0, 0.5),
                    ),
                );
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        x1,
                        0.0,
                        x2,
                        params.w_end_down_t,
                        mod_add(pair.left, 0.0, 0.6),
                        mod_add(pair.right, 0.0, 0.6),
                        mod_add(pair.right, 0.0, 0.6),
                        mod_add(pair.left, 0.0, 0.6),
                    ),
                );
                scene.keyboard_quads += 2;
            } else {
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        x1,
                        params.w_end_up_t,
                        x2,
                        params.top_bar_end,
                        pair.left,
                        pair.right,
                        mod_add(pair.right, 0.0, 0.8),
                        mod_add(pair.left, 0.0, 0.8),
                    ),
                );
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        x1,
                        params.w_end_up_b,
                        x2,
                        params.w_end_up_t,
                        [0.529, 0.529, 0.529, 1.0],
                        [0.529, 0.529, 0.529, 1.0],
                        [0.329, 0.329, 0.329, 1.0],
                        [0.329, 0.329, 0.329, 1.0],
                    ),
                );
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        x1,
                        0.0,
                        x2,
                        params.w_end_up_b,
                        [0.615, 0.615, 0.615, 1.0],
                        [0.615, 0.615, 0.615, 1.0],
                        [0.729, 0.729, 0.729, 1.0],
                        [0.729, 0.729, 0.729, 1.0],
                    ),
                );
                scene.keyboard_quads += 3;
            }

            if n == 60 && self.middle_c {
                let marker_x1 = x1 + width / 4.0;
                let marker_x2 = x2 - width / 4.0;
                let marker_y2 = if key_pressed[n] {
                    params.w_end_down_t + width / 4.0
                } else {
                    params.w_end_up_t + width / 4.0
                };
                let marker_y1 = marker_y2 + width / 2.0 * params.scwidth / params.scwidth.max(1.0);
                scene.push_quad(
                    SceneLayer::WhiteKeys,
                    vertical_gradient_quad(
                        marker_x1,
                        marker_y2,
                        marker_x2,
                        marker_y1,
                        mod_add(pair.left, 0.0, 0.8),
                        mod_add(pair.right, 0.0, 0.8),
                        mod_add(pair.right, 0.0, 0.8),
                        mod_add(pair.left, 0.0, 0.8),
                    ),
                );
            }

            let sep_width = ((arrays.width[n] * params.scwidth) / 20.0).round().max(1.0);
            let sep_left = ((x1 * params.scwidth) - sep_width / 2.0).floor() / params.scwidth;
            let mut sep_right = ((x1 * params.scwidth) + sep_width / 2.0).floor() / params.scwidth;
            if sep_left == sep_right {
                sep_right = (sep_left * params.scwidth + 1.0) / params.scwidth;
            }
            scene.push_quad(
                SceneLayer::WhiteKeys,
                quad(
                    [
                        [sep_left, 0.0],
                        [sep_right, 0.0],
                        [sep_right, params.top_bar_end],
                        [sep_left, params.top_bar_end],
                    ],
                    [
                        [0.0431, 0.0431, 0.0431, 1.0],
                        [0.556, 0.556, 0.556, 1.0],
                        [0.556, 0.556, 0.556, 1.0],
                        [0.0431, 0.0431, 0.0431, 1.0],
                    ],
                ),
            );
            scene.keyboard_quads += 1;
        }
    }

    fn push_black_keys(
        &self,
        scene: &mut ProjectedScene,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        kbfirst: usize,
        kblast: usize,
        key_colors: &[KeyColorPair; MIDI_KEY_COUNT],
        key_pressed: &[bool; MIDI_KEY_COUNT],
        orig_black: KeyColorPair,
    ) {
        for n in kbfirst..kblast {
            if !is_black_key(n as u8) {
                continue;
            }

            let ox1 = arrays.x1[n];
            let width = arrays.width[n];
            let ox2 = ox1 + width;
            let ix1 = ox1 + width / 8.0;
            let ix2 = ox2 - width / 8.0;
            let pair = blend_key_pair(key_colors[n], orig_black);
            let mid = [
                (pair.left[0] + pair.right[0]) / 2.0,
                (pair.left[1] + pair.right[1]) / 2.0,
                (pair.left[2] + pair.right[2]) / 2.0,
                1.0,
            ];

            let top = if key_pressed[n] {
                params.b_key_down_t
            } else {
                params.b_key_up_t
            };
            let bottom = if key_pressed[n] {
                params.b_key_down_b
            } else {
                params.b_key_up_b
            };
            if key_pressed[n] {
                scene.active_keys += 1;
            }

            if key_pressed[n] {
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, params.b_key_u_split_lt],
                            [ix2, params.b_key_u_split_rt],
                            [ix2, top],
                            [ix1, top],
                        ],
                        [
                            mod_add(mid, 0.0, 0.85),
                            mod_add(mid, 0.0, 0.85),
                            mod_add(pair.right, 0.0, 0.85),
                            mod_add(pair.right, 0.0, 0.85),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, params.b_key_u_split_lb],
                            [ix2, params.b_key_u_split_rb],
                            [ix2, params.b_key_u_split_rt],
                            [ix1, params.b_key_u_split_lt],
                        ],
                        [
                            mod_add(mid, 0.0, 0.7),
                            mod_add(mid, 0.0, 0.7),
                            mod_add(mid, 0.0, 0.85),
                            mod_add(mid, 0.0, 0.85),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, bottom],
                            [ix2, bottom],
                            [ix2, params.b_key_u_split_rb],
                            [ix1, params.b_key_u_split_lb],
                        ],
                        [
                            mod_add(pair.left, 0.0, 0.7),
                            mod_add(pair.left, 0.0, 0.7),
                            mod_add(mid, 0.0, 0.7),
                            mod_add(mid, 0.0, 0.7),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ox1, params.b_key_end],
                            [ix1, bottom],
                            [ix1, top],
                            [ox1, params.top_bar_end],
                        ],
                        [
                            mod_add(pair.left, 0.0, 0.7),
                            pair.left,
                            pair.right,
                            mod_add(pair.right, 0.0, 0.7),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix2, bottom],
                            [ox2, params.b_key_end],
                            [ox2, params.top_bar_end],
                            [ix2, top],
                        ],
                        [
                            pair.left,
                            mod_add(pair.left, 0.0, 0.7),
                            mod_add(pair.right, 0.0, 0.7),
                            pair.right,
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ox1, params.b_key_end],
                            [ox2, params.b_key_end],
                            [ix2, bottom],
                            [ix1, bottom],
                        ],
                        [
                            mod_add(pair.left, 0.0, 0.7),
                            mod_add(pair.left, 0.0, 0.7),
                            pair.left,
                            pair.left,
                        ],
                    ),
                );
            } else {
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, params.b_key_u_split_lt],
                            [ix2, params.b_key_u_split_rt],
                            [ix2, top],
                            [ix1, top],
                        ],
                        [
                            mod_add(pair.left, 0.25, 1.0),
                            mod_add(pair.left, 0.25, 1.0),
                            mod_add(pair.right, 0.15, 1.0),
                            mod_add(pair.right, 0.15, 1.0),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, params.b_key_u_split_lb],
                            [ix2, params.b_key_u_split_rb],
                            [ix2, params.b_key_u_split_rt],
                            [ix1, params.b_key_u_split_lt],
                        ],
                        [
                            pair.left,
                            pair.left,
                            mod_add(pair.right, 0.25, 1.0),
                            mod_add(pair.right, 0.25, 1.0),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix1, bottom],
                            [ix2, bottom],
                            [ix2, params.b_key_u_split_rb],
                            [ix1, params.b_key_u_split_lb],
                        ],
                        [pair.left, pair.left, pair.right, pair.right],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ox1, params.b_key_end],
                            [ix1, bottom],
                            [ix1, top],
                            [ox1, params.top_bar_end],
                        ],
                        [
                            pair.left,
                            mod_add(pair.left, 0.3, 1.0),
                            mod_add(pair.right, 0.3, 1.0),
                            pair.right,
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ix2, bottom],
                            [ox2, params.b_key_end],
                            [ox2, params.top_bar_end],
                            [ix2, top],
                        ],
                        [
                            mod_add(pair.left, 0.3, 1.0),
                            pair.left,
                            pair.right,
                            mod_add(pair.right, 0.3, 1.0),
                        ],
                    ),
                );
                scene.push_quad(
                    SceneLayer::BlackKeys,
                    quad(
                        [
                            [ox1, params.b_key_end],
                            [ox2, params.b_key_end],
                            [ix2, bottom],
                            [ix1, bottom],
                        ],
                        [
                            pair.left,
                            pair.left,
                            mod_add(pair.right, 0.3, 1.0),
                            mod_add(pair.right, 0.3, 1.0),
                        ],
                    ),
                );
            }
            scene.keyboard_quads += 6;
        }
    }
}

struct KeyNoteProjection {
    key: usize,
    notes: Vec<NoteInstance>,
    key_color: KeyColorPair,
    key_pressed: bool,
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

fn blend_key_pair(pair: KeyColorPair, base: KeyColorPair) -> KeyColorPair {
    KeyColorPair {
        left: alpha_blend(pair.left, base.left),
        right: alpha_blend(pair.right, base.right),
    }
}

fn alpha_blend(top: [f32; 4], bottom: [f32; 4]) -> [f32; 4] {
    let blend = top[3];
    let inv = 1.0 - blend;
    [
        top[0] * blend + bottom[0] * inv,
        top[1] * blend + bottom[1] * inv,
        top[2] * blend + bottom[2] * inv,
        1.0,
    ]
}
