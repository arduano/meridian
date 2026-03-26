pub mod wgpu;

use bytemuck::{Pod, Zeroable};
use clap::ValueEnum;

use crate::midi::{MIDI_KEY_COUNT, MIDIFileUnion};

const LAYER_COUNT: usize = 7;
const NOTE_DEPTH_STEP: f32 = 1.0 / 16_777_216.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RendererKind {
    Basic,
    Pfa,
}

#[repr(usize)]
#[derive(Clone, Copy, Debug)]
pub enum SceneLayer {
    Background = 0,
    WhiteNotes = 1,
    BlackNotes = 2,
    KeyboardDecor = 3,
    WhiteKeys = 4,
    BlackKeys = 5,
    Overlay = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SceneQuad {
    pub positions: [[f32; 2]; 4],
    pub colors: [[f32; 4]; 4],
    pub depth: f32,
    pub _padding: [f32; 3],
}

#[derive(Clone, Copy)]
pub struct SceneLayout {
    pub renderer: RendererKind,
    pub view_range: f64,
    pub piano_height: f32,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for SceneLayout {
    fn default() -> Self {
        Self {
            renderer: RendererKind::Pfa,
            view_range: 8.0,
            piano_height: 0.151,
            first_key: 0,
            last_key: 127,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }
}

pub struct ProjectedScene {
    layers: [Vec<SceneQuad>; LAYER_COUNT],
    pub notes_black_first: bool,
    pub visible_notes: usize,
    pub active_keys: usize,
    pub note_quads: usize,
    pub keyboard_quads: usize,
}

impl Default for ProjectedScene {
    fn default() -> Self {
        Self {
            layers: std::array::from_fn(|_| Vec::new()),
            notes_black_first: false,
            visible_notes: 0,
            active_keys: 0,
            note_quads: 0,
            keyboard_quads: 0,
        }
    }
}

impl ProjectedScene {
    pub fn clear(&mut self) {
        for layer in &mut self.layers {
            layer.clear();
        }
        self.notes_black_first = false;
        self.visible_notes = 0;
        self.active_keys = 0;
        self.note_quads = 0;
        self.keyboard_quads = 0;
    }

    pub fn push_quad(&mut self, layer: SceneLayer, quad: SceneQuad) {
        self.layers[layer as usize].push(quad);
    }

    pub fn layer(&self, layer: SceneLayer) -> &[SceneQuad] {
        &self.layers[layer as usize]
    }

    pub fn total_quads(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }

    pub fn total_vertices(&self) -> usize {
        self.total_quads() * 6
    }
}

pub fn project_scene(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    project_scene_into(midi, current_time, layout, &mut scene);
    scene
}

pub fn project_scene_into(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    scene.clear();
    match layout.renderer {
        RendererKind::Basic => BasicProjector.project_into(midi, current_time, layout, scene),
        RendererKind::Pfa => {
            PfaProjector::default().project_into(midi, current_time, layout, scene)
        }
    }
}

trait SceneProjector {
    fn project_into(
        &self,
        midi: &mut MIDIFileUnion,
        current_time: f64,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    );
}

struct BasicProjector;

impl SceneProjector for BasicProjector {
    fn project_into(
        &self,
        midi: &mut MIDIFileUnion,
        current_time: f64,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = false;
        let views = midi.get_current_column_views(current_time, layout.view_range);
        let first_key = layout.first_key.min(layout.last_key) as usize;
        let last_key = layout.last_key.max(layout.first_key) as usize;
        let view_range = views.range().length() as f32;
        let piano_height = layout.piano_height;
        let key_width = 1.0 / ((last_key - first_key + 1) as f32);

        for key in first_key..=last_key.min(MIDI_KEY_COUNT - 1) {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let mut key_color = None;
            let column = views.get_column(key);

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= view_range {
                    return;
                }

                let bottom =
                    piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
                let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
                let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });
                let border = shade(color, if is_black { 0.18 } else { -0.18 });
                let inset = key_width * if is_black { 0.10 } else { 0.06 };

                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(x1, bottom, x2, top, border, border, border, border),
                );
                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(
                        x1 + inset,
                        bottom + 0.003,
                        x2 - inset,
                        (top - 0.003).max(bottom + 0.003),
                        color,
                        color,
                        shade(color, 0.08),
                        shade(color, 0.08),
                    ),
                );

                if note.start <= 0.0 && end > 0.0 {
                    key_color = Some(color);
                }
                scene.visible_notes += 1;
                scene.note_quads += 2;
            });

            let key_top = if is_black {
                piano_height * 0.62
            } else {
                piano_height
            };
            let base = if is_black {
                [0.08, 0.16, 0.09, 1.0]
            } else {
                [0.74, 0.96, 0.76, 1.0]
            };
            let fill = key_color
                .map(|color| mix(base, color, if is_black { 0.75 } else { 0.45 }))
                .unwrap_or(base);
            let border = if is_black {
                shade(fill, 0.12)
            } else {
                shade(fill, -0.18)
            };
            if key_color.is_some() {
                scene.active_keys += 1;
            }

            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(x1, 0.0, x2, key_top, border, border, border, border),
            );
            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(
                    x1 + key_width * if is_black { 0.08 } else { 0.03 },
                    piano_height * 0.012,
                    x2 - key_width * if is_black { 0.08 } else { 0.03 },
                    key_top - piano_height * 0.012,
                    fill,
                    fill,
                    fill,
                    fill,
                ),
            );
            scene.keyboard_quads += 2;
        }
    }
}

#[derive(Clone, Copy)]
struct PfaProjector {
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
        midi: &mut MIDIFileUnion,
        current_time: f64,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = self.black_notes_above;
        let views = midi.get_current_column_views(current_time, layout.view_range);
        let params = PfaLayoutParams::new(layout, views.range().length() as f32, self.border_width);
        let mut note_depth_index = 0_u32;
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
                &mut note_depth_index,
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
                &mut note_depth_index,
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
                &mut note_depth_index,
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

    fn note_y1(&self, note_end: f32, has_ended: bool) -> f32 {
        if has_ended {
            1.0 - (self.view_range - note_end) * self.note_pos_factor
        } else {
            1.0
        }
    }

    fn note_y2(&self, note_start: f32) -> f32 {
        1.0 - (self.view_range - note_start) * self.note_pos_factor
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
        views: &crate::midi::MIDIFileViewsUnion<'_>,
        params: &PfaLayoutParams,
        arrays: &KeyPositionArrays,
        first_note: usize,
        last_note: usize,
        only_black: bool,
        key_colors: &mut [KeyColorPair; MIDI_KEY_COUNT],
        key_pressed: &mut [bool; MIDI_KEY_COUNT],
        note_depth_index: &mut u32,
    ) {
        for k in first_note..last_note.min(MIDI_KEY_COUNT) {
            if only_black != is_black_key(k as u8) {
                continue;
            }
            let x1 = arrays.x1[k];
            let width = arrays.width[k];
            let x2 = x1 + width;
            let is_black = is_black_key(k as u8);
            let column = views.get_column(k);
            let layer = if is_black {
                SceneLayer::BlackNotes
            } else {
                SceneLayer::WhiteNotes
            };

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= params.view_range {
                    return;
                }

                let left = note.color.to_rgba(1.0);
                let right = note.color.to_rgba(1.0);
                if note.start <= 0.0 && end > 0.0 {
                    if is_black && self.black_notes_above {
                        key_colors[k] = KeyColorPair { left, right };
                    } else {
                        key_colors[k].left = alpha_blend(left, key_colors[k].left);
                        key_colors[k].right = alpha_blend(right, key_colors[k].right);
                    }
                    key_pressed[k] = true;
                }

                let top = params.note_y1(end.min(params.view_range), true);
                let bottom = params.note_y2(note.start.max(0.0));
                let base_depth = (*note_depth_index as f32) * NOTE_DEPTH_STEP * 2.0;
                let outer_depth = base_depth + NOTE_DEPTH_STEP;
                let inner_depth = base_depth;
                *note_depth_index += 1;

                scene.push_quad(
                    layer,
                    vertical_gradient_quad_with_depth(
                        x1,
                        bottom,
                        x2,
                        top,
                        mod_add(right, 0.0, 0.2),
                        mod_add(left, 0.0, 0.2),
                        mod_add(left, 0.0, 0.2),
                        mod_add(right, 0.0, 0.2),
                        outer_depth,
                    ),
                );
                scene.note_quads += 1;

                if top - bottom > params.padding_y * 2.0 {
                    let ix1 = x1 + params.padding_x;
                    let ix2 = x2 - params.padding_x;
                    let iy_top = top - params.padding_y;
                    let iy_bottom = bottom + params.padding_y;
                    scene.push_quad(
                        layer,
                        vertical_gradient_quad_with_depth(
                            ix1,
                            iy_bottom,
                            ix2,
                            iy_top,
                            mod_add(left, 0.0, 0.5),
                            mod_add(left, 0.0, 1.0),
                            mod_add(right, 0.0, 1.0),
                            mod_add(right, 0.0, 0.5),
                            inner_depth,
                        ),
                    );
                    scene.note_quads += 1;
                }

                scene.visible_notes += 1;
            });
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

fn vertical_gradient_quad(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    c_bl: [f32; 4],
    c_br: [f32; 4],
    c_tr: [f32; 4],
    c_tl: [f32; 4],
) -> SceneQuad {
    quad(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [c_bl, c_br, c_tr, c_tl],
    )
}

fn quad(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4]) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth: 0.5,
        _padding: [0.0; 3],
    }
}

fn quad_with_depth(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4], depth: f32) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth,
        _padding: [0.0; 3],
    }
}

fn vertical_gradient_quad_with_depth(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    c_bl: [f32; 4],
    c_br: [f32; 4],
    c_tr: [f32; 4],
    c_tl: [f32; 4],
    depth: f32,
) -> SceneQuad {
    quad_with_depth(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [c_bl, c_br, c_tr, c_tl],
        depth,
    )
}

fn mod_add(color: [f32; 4], add: f32, mul: f32) -> [f32; 4] {
    [
        (color[0] * mul + add).clamp(0.0, 1.0),
        (color[1] * mul + add).clamp(0.0, 1.0),
        (color[2] * mul + add).clamp(0.0, 1.0),
        color[3],
    ]
}

fn shade(color: [f32; 4], amount: f32) -> [f32; 4] {
    [
        (color[0] + amount).clamp(0.0, 1.0),
        (color[1] + amount).clamp(0.0, 1.0),
        (color[2] + amount).clamp(0.0, 1.0),
        color[3],
    ]
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn is_black_key(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}
