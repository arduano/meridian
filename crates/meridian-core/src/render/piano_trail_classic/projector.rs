use glam::{Mat4, Vec3};
use rayon::prelude::*;

use crate::{
    midi::views::MIDIFileViewsUnion,
    render::{
        SceneLayout,
        shared::{PianoTrailClassicSceneConfig, ProjectedScene, alpha_blend, is_black_key},
    },
};

use super::{
    layout::PianoTrailClassicLayout,
    model::{PianoTrailClassicQuadInstance, PianoTrailClassicScene},
    physics::PianoTrailClassicPhysicsState,
};

const WHITE_KEY_LEN: f32 = 5.0;
const BLACK_KEY_LEN: f32 = 6.9;
const WHITE_KEY_LENFAC: f32 = 0.69;
const KEY_X_SQUEEZE: f32 = 0.95;
const WHITE_KEY_Y_DROP: f32 = 0.3;
const BLACK_KEY_Y_LIFT: f32 = 1.2;
const AURA_REFERENCE_FPS: f32 = 60.0;
const AURA_MAX_SECONDS: f32 = 1.0;
#[derive(Clone, Copy)]
struct VisibleNote {
    key: usize,
    start: f32,
    end: f32,
    unclamped_end: f32,
    left: [f32; 4],
    right: [f32; 4],
    active: bool,
    has_ended: bool,
}

#[derive(Clone, Copy, Default)]
struct KeyState {
    left: [f32; 4],
    right: [f32; 4],
    press: f32,
    aura: f32,
}

pub fn project_piano_trail_classic_scene(
    config: &PianoTrailClassicSceneConfig,
    physics: Option<&crate::render::ScenePhysicsState>,
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    let first_key = layout.first_key.min(layout.last_key) as usize;
    let last_key = layout.last_key.max(layout.first_key) as usize + 1;
    let key_layout = PianoTrailClassicLayout::new(first_key, last_key, config);
    let mut piano_trail_classic = scene.take_piano_trail_classic().unwrap_or_default();
    piano_trail_classic.clear();

    let notes_by_key = collect_visible_notes(config, views, &key_layout, layout.view_range as f32);
    let piano_trail_classic_physics = match physics {
        Some(crate::render::ScenePhysicsState::PianoTrailClassic(state)) => Some(state),
        _ => None,
    };
    let key_order = key_render_order(config, &key_layout);
    let world_to_camera = build_world_to_camera(config);

    let mut key_state = vec![KeyState::default(); 256];
    for notes in &notes_by_key {
        for note in notes {
            scene.visible_notes += 1;
            scene.note_quads += 1 + usize::from(config.box_notes) * 2;
            if note.active {
                let state = &mut key_state[note.key];
                state.left = alpha_blend(state.left, note.left);
                state.right = alpha_blend(state.right, note.right);
                state.aura = state.aura.max(aura_size(*note));
            }
        }
    }

    if let Some(physics) = piano_trail_classic_physics {
        apply_key_press_state(&mut key_state, physics);
    }

    for &key in &key_order {
        let notes = &notes_by_key[key - key_layout.first_key];
        if config.box_notes {
            for note in notes {
                emit_note_cap(
                    &mut piano_trail_classic,
                    &key_layout,
                    config,
                    layout.view_range as f32,
                    *note,
                );
            }
            for note in notes.iter().rev() {
                emit_note_side(
                    &mut piano_trail_classic,
                    &key_layout,
                    config,
                    &world_to_camera,
                    layout.view_range as f32,
                    *note,
                );
            }
        }
        for note in notes.iter().rev() {
            emit_note_front(
                &mut piano_trail_classic,
                &key_layout,
                config,
                layout.view_range as f32,
                *note,
            );
        }
    }

    if config.show_keyboard {
        emit_keyboard(
            &mut piano_trail_classic,
            &key_layout,
            config,
            &key_state,
            &mut scene.active_keys,
            &mut scene.keyboard_quads,
        );
    }
    if config.aura_enabled {
        emit_aura(&mut piano_trail_classic, &key_layout, config, &key_state);
    }

    scene.set_piano_trail_classic(piano_trail_classic);
}

fn collect_visible_notes(
    config: &PianoTrailClassicSceneConfig,
    views: &MIDIFileViewsUnion<'_>,
    key_layout: &PianoTrailClassicLayout,
    view_range: f32,
) -> Vec<Vec<VisibleNote>> {
    let render_start = -view_range * config.viewback;
    (key_layout.first_key..key_layout.last_key_exclusive)
        .into_par_iter()
        .map(|key| {
            let column = views.get_column(key);
            let iter = column.iterate_displaced_notes();
            let mut notes = Vec::with_capacity(iter.len());
            for note in iter {
                let start = note.start;
                let unclamped_end = start + note.len;
                let mut end = unclamped_end;
                if end < render_start || start >= view_range {
                    continue;
                }
                let mut clamped_start = start.max(render_start);
                if config.eat_notes {
                    clamped_start = clamped_start.max(0.0);
                    end = end.max(0.0);
                }
                notes.push(VisibleNote {
                    key,
                    start: clamped_start,
                    end: end.min(view_range),
                    unclamped_end,
                    left: note.color.left.to_rgba(1.0),
                    right: note.color.right.to_rgba(1.0),
                    active: note.start <= 0.0 && end > 0.0,
                    has_ended: unclamped_end <= view_range,
                });
            }
            notes
        })
        .collect()
}

fn key_render_order(config: &PianoTrailClassicSceneConfig, key_layout: &PianoTrailClassicLayout) -> Vec<usize> {
    let mut keys: Vec<usize> = (key_layout.first_key..key_layout.last_key_exclusive).collect();
    let world_to_camera = build_world_to_camera(config);
    keys.sort_by(|&a, &b| {
        let a_center = key_layout.key_x1(a) + key_layout.key_width(a) * 0.5;
        let b_center = key_layout.key_x1(b) + key_layout.key_width(b) * 0.5;
        let a_view = world_to_camera.transform_point3(Vec3::new(a_center, 0.0, 0.0));
        let b_view = world_to_camera.transform_point3(Vec3::new(b_center, 0.0, 0.0));
        a_view
            .z
            .partial_cmp(&b_view.z)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b_view
                    .x
                    .abs()
                    .partial_cmp(&a_view.x.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    keys
}

fn emit_note_side(
    scene: &mut PianoTrailClassicScene,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    world_to_camera: &Mat4,
    view_range: f32,
    note: VisibleNote,
) {
    let base_x1 = key_layout.key_x1(note.key);
    let width = key_layout.key_width(note.key);
    let z1 = note.end * (config.viewdist / view_range.max(0.0001));
    let z2 = note.start * (config.viewdist / view_range.max(0.0001));

    let factor = active_factor(note, config);
    let mut side_x = choose_camera_facing_x(
        base_x1,
        base_x1 + width,
        -width * 0.5,
        (z1 + z2) * 0.5,
        world_to_camera,
    );
    let mut side_shade = 0.0;
    if note.active {
        if config.notes_change_tint {
            side_shade += factor * 0.7;
        }
        if config.notes_change_size {
            if side_x > 0.0 {
                side_x -= width * 0.3 * factor;
            } else {
                side_x += width * 0.3 * factor;
            }
        }
    }
    if config.light_shade {
        side_shade += 0.2;
    } else {
        side_shade -= 0.3;
    }
    let side_left = shaded(note.left, side_shade);
    let side_right = shaded(note.right, side_shade);
    push_quad_instance(
        &mut scene.note_quads,
        [side_x, 0.0, z2],
        [side_x, 0.0, z1],
        [side_x, -width, z1],
        [side_x, -width, z2],
        side_left,
        side_left,
        side_right,
        side_right,
    );
}

fn choose_camera_facing_x(x1: f32, x2: f32, y: f32, z: f32, world_to_camera: &Mat4) -> f32 {
    let p1 = world_to_camera.transform_point3(Vec3::new(x1, y, z));
    let p2 = world_to_camera.transform_point3(Vec3::new(x2, y, z));
    if p1.length_squared() <= p2.length_squared() {
        x1
    } else {
        x2
    }
}

fn build_world_to_camera(config: &PianoTrailClassicSceneConfig) -> Mat4 {
    Mat4::from_rotation_x(config.cam_ang)
        * Mat4::from_rotation_y(config.cam_rot)
        * Mat4::from_rotation_z(config.cam_spin)
        * Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0))
        * Mat4::from_translation(Vec3::new(
            config.view_pan,
            -config.view_height,
            config.view_offset,
        ))
}

fn emit_note_cap(
    scene: &mut PianoTrailClassicScene,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    view_range: f32,
    note: VisibleNote,
) {
    let base_x1 = key_layout.key_x1(note.key);
    let width = key_layout.key_width(note.key);
    let base_x2 = base_x1 + width;
    let z1 = note.end * (config.viewdist / view_range.max(0.0001));
    let z2 = note.start * (config.viewdist / view_range.max(0.0001));
    let (cap_x1, cap_x2, cap_shade) = cap_or_front_shape(base_x1, base_x2, width, note, config);
    let mut cap_z = z2;
    if (config.vertical_notes && cap_z < config.view_height)
        || (!config.vertical_notes && cap_z < -config.view_offset)
    {
        cap_z = z1;
    }
    let cap_left = shaded(note.left, cap_shade - 0.2);
    let cap_right = shaded(note.right, cap_shade - 0.2);
    push_quad_instance(
        &mut scene.note_quads,
        [cap_x2, -width, cap_z],
        [cap_x2, 0.0, cap_z],
        [cap_x1, 0.0, cap_z],
        [cap_x1, -width, cap_z],
        cap_right,
        cap_right,
        cap_left,
        cap_left,
    );
}

fn emit_note_front(
    scene: &mut PianoTrailClassicScene,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    view_range: f32,
    note: VisibleNote,
) {
    let base_x1 = key_layout.key_x1(note.key);
    let width = key_layout.key_width(note.key);
    let base_x2 = base_x1 + width;
    let z1 = note.end * (config.viewdist / view_range.max(0.0001));
    let z2 = note.start * (config.viewdist / view_range.max(0.0001));
    let (front_x1, front_x2, front_shade) =
        cap_or_front_shape(base_x1, base_x2, width, note, config);
    let front_left = shaded(note.left, front_shade);
    let front_right = shaded(note.right, front_shade);
    push_quad_instance(
        &mut scene.note_quads,
        [front_x2, 0.0, z2],
        [front_x2, 0.0, z1],
        [front_x1, 0.0, z1],
        [front_x1, 0.0, z2],
        front_right,
        front_right,
        front_left,
        front_left,
    );
}

fn active_factor(note: VisibleNote, config: &PianoTrailClassicSceneConfig) -> f32 {
    if !note.active {
        return 0.0;
    }
    config.note_down_speed.clamp(0.0, 1.0) * 0.5
}

fn aura_size(note: VisibleNote) -> f32 {
    if !note.active {
        return 0.0;
    }
    let frames_since_start = (-note.start).max(0.0) * AURA_REFERENCE_FPS;
    let burst = (10.0 - frames_since_start).max(0.0).powi(2) / 600.0;
    let base = if note.has_ended {
        let len = (note.unclamped_end - note.start).clamp(1.0e-6, AURA_MAX_SECONDS);
        let offset = note.unclamped_end.clamp(0.0, AURA_MAX_SECONDS);
        0.5 * (offset / len).powf(0.3)
    } else {
        0.5
    };
    base + burst
}

fn cap_or_front_shape(
    base_x1: f32,
    base_x2: f32,
    width: f32,
    note: VisibleNote,
    config: &PianoTrailClassicSceneConfig,
) -> (f32, f32, f32) {
    let mut x1 = base_x1;
    let mut x2 = base_x2;
    let mut shade = 0.0;
    if note.active {
        let factor = active_factor(note, config);
        if config.notes_change_tint {
            shade += factor * 0.7;
        }
        if config.notes_change_size {
            x1 -= width * 0.3 * factor;
            x2 += width * 0.3 * factor;
        }
    }
    (x1, x2, shade)
}

fn emit_keyboard(
    scene: &mut PianoTrailClassicScene,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    key_state: &[KeyState],
    active_keys: &mut usize,
    keyboard_quads: &mut usize,
) {
    for key in key_layout.first_key..key_layout.last_key_exclusive {
        let pressed = key_state[key].press > 0.0;
        if pressed {
            *active_keys += 1;
        }
        let base_left = if is_black_key(key as u8) {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        let base_right = base_left;
        let left = blend_key_tint(base_left, key_state[key].left);
        let right = blend_key_tint(base_right, key_state[key].right);
        if is_black_key(key as u8) {
            emit_black_key(
                &mut scene.black_key_quads,
                key_layout,
                config,
                key,
                left,
                right,
                key_state[key].press,
            );
            *keyboard_quads += 8;
        } else {
            emit_white_key(
                &mut scene.white_key_quads,
                key_layout,
                config,
                key,
                left,
                right,
                key_state[key].press,
            );
            *keyboard_quads += 13;
        }
    }
}

fn emit_aura(
    scene: &mut PianoTrailClassicScene,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    key_state: &[KeyState],
) {
    for key in key_layout.first_key..key_layout.last_key_exclusive {
        let aura = key_state[key].aura;
        if aura <= 0.0 {
            continue;
        }
        let (mut x1, mut x2) = if !is_black_key(key as u8) && config.same_width_notes {
            key_layout.expanded_white_key_span(key)
        } else {
            let x1 = key_layout.key_x1(key);
            (x1, x1 + key_layout.key_width(key))
        };
        let middle = (x1 + x2) * 0.5;
        let size = key_layout.circle_radius * 12.0 * aura.max(0.25);
        x1 = middle - size;
        x2 = middle + size;
        let y1 = size;
        let y2 = -size;
        let left = scale_alpha(key_state[key].left, config.aura_strength);
        let right = scale_alpha(key_state[key].right, config.aura_strength);
        push_aura_quad(
            scene,
            [x1, y1, 0.0],
            [x1, y2, 0.0],
            [x2, y2, 0.0],
            [x2, y1, 0.0],
            left,
            right,
        );
    }
}

fn emit_white_key(
    out: &mut Vec<PianoTrailClassicQuadInstance>,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    key: usize,
    left: [f32; 4],
    right: [f32; 4],
    press: f32,
) {
    let pitch = white_pitch_index(key as u8);
    let (offset_left, offset_right) = white_key_offsets(key_layout, key, pitch);
    let (base_x, base_x2) = if config.same_width_notes {
        key_layout.expanded_white_key_span(key)
    } else {
        let x1 = key_layout.key_x1(key);
        (x1, x1 + key_layout.key_width(key))
    };
    let width = base_x2 - base_x;
    let width2 = if config.same_width_notes {
        key_layout.key_width(key_layout.first_key) * 2.0
    } else {
        width
    };
    let scale_x = width;
    let scale_y = if config.same_width_notes {
        width2 * 0.9
    } else {
        width2
    };
    let scale_z = if config.same_width_notes {
        width2 * 1.01
    } else {
        width2
    };
    let black_end = WHITE_KEY_LEN * WHITE_KEY_LENFAC;

    let quads = [
        (
            [0.0, 0.0, WHITE_KEY_LEN],
            [0.0, 0.7, WHITE_KEY_LEN],
            [1.0, 0.7, WHITE_KEY_LEN],
            [1.0, 0.0, WHITE_KEY_LEN],
            [0.7, 0.8, 0.8, 0.7],
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            [0.0, 0.7, WHITE_KEY_LEN],
            [0.0, 1.0, WHITE_KEY_LEN],
            [1.0, 1.0, WHITE_KEY_LEN],
            [1.0, 0.7, WHITE_KEY_LEN],
            [0.6, 0.6, 0.6, 0.6],
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            [1.0, 1.0, black_end],
            [1.0, 1.0, WHITE_KEY_LEN],
            [0.0, 1.0, WHITE_KEY_LEN],
            [0.0, 1.0, black_end],
            [1.0, 1.0, 1.0, 1.0],
            [WHITE_KEY_LENFAC, 1.0, 1.0, WHITE_KEY_LENFAC],
        ),
        (
            [0.0, 1.0, WHITE_KEY_LEN],
            [0.03, 0.95, WHITE_KEY_LEN + 0.1],
            [0.97, 0.95, WHITE_KEY_LEN + 0.1],
            [1.0, 1.0, WHITE_KEY_LEN],
            [1.0, 0.9, 0.9, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            [0.0, 0.9, WHITE_KEY_LEN + 0.07],
            [0.03, 0.95, WHITE_KEY_LEN + 0.1],
            [0.97, 0.95, WHITE_KEY_LEN + 0.1],
            [1.0, 0.9, WHITE_KEY_LEN + 0.07],
            [0.9, 0.9, 0.9, 0.9],
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            [0.0, 1.0, black_end],
            [0.0, 1.0, WHITE_KEY_LEN],
            [0.0, 0.0, WHITE_KEY_LEN],
            [0.0, 0.0, black_end],
            [0.6, 0.6, 0.6, 0.6],
            [WHITE_KEY_LENFAC, 1.0, 1.0, WHITE_KEY_LENFAC],
        ),
        (
            [1.0, 1.0, black_end],
            [1.0, 1.0, WHITE_KEY_LEN],
            [1.0, 0.0, WHITE_KEY_LEN],
            [1.0, 0.0, black_end],
            [0.6, 0.6, 0.6, 0.6],
            [WHITE_KEY_LENFAC, 1.0, 1.0, WHITE_KEY_LENFAC],
        ),
        (
            [offset_left, 1.0, 0.0],
            [offset_left, 1.0, black_end],
            [offset_right, 1.0, black_end],
            [offset_right, 1.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.0, WHITE_KEY_LENFAC, WHITE_KEY_LENFAC, 0.0],
        ),
        (
            [offset_left, 0.0, 0.0],
            [offset_left, 1.0, 0.0],
            [offset_right, 1.0, 0.0],
            [offset_right, 0.0, 0.0],
            [0.8, 0.8, 0.8, 0.8],
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            [offset_left, 1.0, 0.0],
            [offset_left, 1.0, black_end],
            [offset_left, 0.0, black_end],
            [offset_left, 0.0, 0.0],
            [0.6, 0.6, 0.6, 0.6],
            [0.0, WHITE_KEY_LENFAC, WHITE_KEY_LENFAC, 0.0],
        ),
        (
            [offset_right, 1.0, 0.0],
            [offset_right, 1.0, black_end],
            [offset_right, 0.0, black_end],
            [offset_right, 0.0, 0.0],
            [0.6, 0.6, 0.6, 0.6],
            [0.0, WHITE_KEY_LENFAC, WHITE_KEY_LENFAC, 0.0],
        ),
        (
            [0.0, 1.0, black_end],
            [offset_left, 1.0, black_end],
            [offset_left, 0.0, black_end],
            [0.0, 0.0, black_end],
            [0.6, 0.6, 0.6, 0.6],
            [
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
            ],
        ),
        (
            [1.0, 1.0, black_end],
            [offset_right, 1.0, black_end],
            [offset_right, 0.0, black_end],
            [1.0, 0.0, black_end],
            [0.6, 0.6, 0.6, 0.6],
            [
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
                WHITE_KEY_LENFAC,
            ],
        ),
    ];

    for (a_pos, b_pos, c_pos, d_pos, brightness, blend) in quads {
        let a = white_key_color(left, right, brightness[0], blend[0]);
        let b = white_key_color(left, right, brightness[1], blend[1]);
        let c = white_key_color(left, right, brightness[2], blend[2]);
        let d = white_key_color(left, right, brightness[3], blend[3]);
        push_quad_instance(
            out,
            transform_white_key(
                a_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                press,
                config.tilt_keys,
            ),
            transform_white_key(
                b_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                press,
                config.tilt_keys,
            ),
            transform_white_key(
                c_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                press,
                config.tilt_keys,
            ),
            transform_white_key(
                d_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                press,
                config.tilt_keys,
            ),
            a,
            b,
            c,
            d,
        );
    }
}

fn emit_black_key(
    out: &mut Vec<PianoTrailClassicQuadInstance>,
    key_layout: &PianoTrailClassicLayout,
    config: &PianoTrailClassicSceneConfig,
    key: usize,
    left: [f32; 4],
    right: [f32; 4],
    press: f32,
) {
    let base_x = key_layout.key_x1(key);
    let width = key_layout.key_width(key);
    let scale_x = width;
    let vert_offset = if config.same_width_notes { 1.2 } else { 1.1 };
    let scale_y = width / vert_offset;
    let scale_z = width;
    let quads = [
        (
            [0.0, 0.0, BLACK_KEY_LEN],
            [0.0, 1.0, BLACK_KEY_LEN - 1.0],
            [1.0, 1.0, BLACK_KEY_LEN - 1.0],
            [1.0, 0.0, BLACK_KEY_LEN],
            [0.9, 0.95, 0.95, 0.9],
            [1.0, 1.0, 1.0, 1.0],
        ),
        (
            [0.0, 1.0, BLACK_KEY_LEN - 1.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, BLACK_KEY_LEN - 1.0],
            [1.0, 0.94, 0.94, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ),
        (
            [0.0, 0.0, 0.0],
            [0.0, 0.0, BLACK_KEY_LEN],
            [0.0, 1.0, BLACK_KEY_LEN - 1.0],
            [0.0, 1.0, 0.0],
            [0.8, 0.8, 0.9, 0.8],
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            [1.0, 0.0, 0.0],
            [1.0, 0.0, BLACK_KEY_LEN],
            [1.0, 1.0, BLACK_KEY_LEN - 1.0],
            [1.0, 1.0, 0.0],
            [0.8, 0.8, 0.9, 0.8],
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            [0.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.9, 0.9, 0.9, 0.9],
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            [0.0, 0.0, 0.0],
            [0.0, 0.0, BLACK_KEY_LEN],
            [0.0, -1.0, BLACK_KEY_LEN],
            [0.0, -1.0, 0.0],
            [0.8, 0.8, 0.8, 0.8],
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            [1.0, 0.0, 0.0],
            [1.0, 0.0, BLACK_KEY_LEN],
            [1.0, -1.0, BLACK_KEY_LEN],
            [1.0, -1.0, 0.0],
            [0.8, 0.8, 0.8, 0.8],
            [0.0, 1.0, 1.0, 0.0],
        ),
        (
            [0.0, 0.0, BLACK_KEY_LEN],
            [0.0, -1.0, BLACK_KEY_LEN],
            [1.0, -1.0, BLACK_KEY_LEN],
            [1.0, 0.0, BLACK_KEY_LEN],
            [0.9, 1.0, 1.0, 0.9],
            [1.0, 1.0, 1.0, 1.0],
        ),
    ];

    for (a_pos, b_pos, c_pos, d_pos, brightness, blend) in quads {
        let a = black_key_color(left, right, brightness[0], blend[0]);
        let b = black_key_color(left, right, brightness[1], blend[1]);
        let c = black_key_color(left, right, brightness[2], blend[2]);
        let d = black_key_color(left, right, brightness[3], blend[3]);
        push_quad_instance(
            out,
            transform_black_key(
                a_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                vert_offset,
                press,
                config.tilt_keys,
            ),
            transform_black_key(
                b_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                vert_offset,
                press,
                config.tilt_keys,
            ),
            transform_black_key(
                c_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                vert_offset,
                press,
                config.tilt_keys,
            ),
            transform_black_key(
                d_pos,
                base_x,
                scale_x,
                scale_y,
                scale_z,
                vert_offset,
                press,
                config.tilt_keys,
            ),
            a,
            b,
            c,
            d,
        );
    }
}

fn white_key_offsets(key_layout: &PianoTrailClassicLayout, key: usize, pitch: usize) -> (f32, f32) {
    let offsets = [
        (0.0, 0.6),
        (0.2, 0.8),
        (0.4, 1.0),
        (0.0, 0.55),
        (0.15, 0.7),
        (0.3, 0.85),
        (0.45, 1.0),
    ];
    let (mut left, mut right) = offsets[pitch];
    if key == key_layout.first_key {
        left = 0.0;
    }
    if key + 1 == key_layout.last_key_exclusive {
        right = 1.0;
    }
    (left, right)
}

fn white_pitch_index(key: u8) -> usize {
    match key % 12 {
        0 => 0,
        2 => 1,
        4 => 2,
        5 => 3,
        7 => 4,
        9 => 5,
        11 => 6,
        _ => 0,
    }
}

fn apply_key_press_state(key_state: &mut [KeyState], physics: &PianoTrailClassicPhysicsState) {
    for (state, press) in key_state.iter_mut().zip(physics.key_press.iter()) {
        state.press = *press;
    }
}

fn squeeze_local_x(x: f32) -> f32 {
    (x - 0.5) * KEY_X_SQUEEZE + 0.5
}

fn apply_key_motion_world(
    position: [f32; 3],
    press: f32,
    tilt_keys: bool,
    down_divisor: f32,
    pivot_y: f32,
    pivot_z: f32,
) -> [f32; 3] {
    if press <= 0.0 {
        return position;
    }
    if tilt_keys {
        rotate_local_x_around(position, -press / 20.0, pivot_y, pivot_z)
    } else {
        [position[0], position[1] - press / down_divisor, position[2]]
    }
}

fn rotate_local_x_around(position: [f32; 3], angle: f32, pivot_y: f32, pivot_z: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    let y = position[1] - pivot_y;
    let z_rel = position[2] - pivot_z;
    let y2 = y * cos - z_rel * sin + pivot_y;
    let z2 = y * sin + z_rel * cos + pivot_z;
    [position[0], y2, z2]
}

fn transform_white_key(
    position: [f32; 3],
    base_x: f32,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    press: f32,
    tilt_keys: bool,
) -> [f32; 3] {
    let pivot_y = -WHITE_KEY_Y_DROP * scale_y;
    let transformed = [
        base_x + squeeze_local_x(position[0]) * scale_x,
        position[1] * scale_y + pivot_y,
        -position[2] * scale_z,
    ];
    apply_key_motion_world(transformed, press, tilt_keys, 2.0, pivot_y, 0.0)
}

fn transform_black_key(
    position: [f32; 3],
    base_x: f32,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    vert_offset: f32,
    press: f32,
    tilt_keys: bool,
) -> [f32; 3] {
    let pivot_y = vert_offset * (BLACK_KEY_Y_LIFT / 1.2) * scale_y;
    let transformed = [
        base_x + squeeze_local_x(position[0]) * scale_x,
        position[1] * scale_y + pivot_y,
        -position[2] * scale_z,
    ];
    apply_key_motion_world(transformed, press, tilt_keys, 1.2, pivot_y, 0.0)
}

fn white_key_color(left: [f32; 4], right: [f32; 4], brightness: f32, blend: f32) -> [f32; 4] {
    [
        (left[0] * blend + right[0] * (1.0 - blend)) * brightness,
        (left[1] * blend + right[1] * (1.0 - blend)) * brightness,
        (left[2] * blend + right[2] * (1.0 - blend)) * brightness,
        1.0,
    ]
}

fn black_key_color(left: [f32; 4], right: [f32; 4], brightness: f32, blend: f32) -> [f32; 4] {
    let tint = [
        left[0] * blend + right[0] * (1.0 - blend),
        left[1] * blend + right[1] * (1.0 - blend),
        left[2] * blend + right[2] * (1.0 - blend),
    ];
    [
        (1.0 - brightness + tint[0] * brightness).clamp(0.0, 1.0),
        (1.0 - brightness + tint[1] * brightness).clamp(0.0, 1.0),
        (1.0 - brightness + tint[2] * brightness).clamp(0.0, 1.0),
        1.0,
    ]
}

fn push_quad_instance(
    out: &mut Vec<PianoTrailClassicQuadInstance>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    ca: [f32; 4],
    cb: [f32; 4],
    cc: [f32; 4],
    cd: [f32; 4],
) {
    out.push(PianoTrailClassicQuadInstance {
        positions: [a, b, c, d],
        colors: [ca, cb, cc, cd],
    });
}

fn push_aura_quad(
    scene: &mut PianoTrailClassicScene,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    left: [f32; 4],
    right: [f32; 4],
) {
    scene.aura_quads.push(PianoTrailClassicQuadInstance {
        positions: [a, b, c, d],
        colors: [left, left, right, right],
    });
}

fn mix(base: [f32; 4], tint: [f32; 4], amount: f32) -> [f32; 4] {
    let amount = amount.clamp(0.0, 1.0);
    [
        tint[0] * amount + base[0] * (1.0 - amount),
        tint[1] * amount + base[1] * (1.0 - amount),
        tint[2] * amount + base[2] * (1.0 - amount),
        1.0,
    ]
}

fn shaded(color: [f32; 4], shade: f32) -> [f32; 4] {
    [
        (color[0] + shade).clamp(0.0, 1.0),
        (color[1] + shade).clamp(0.0, 1.0),
        (color[2] + shade).clamp(0.0, 1.0),
        color[3],
    ]
}

fn scale_alpha(color: [f32; 4], strength: f32) -> [f32; 4] {
    [
        color[0] * strength,
        color[1] * strength,
        color[2] * strength,
        color[3] * strength,
    ]
}

fn blend_key_tint(base: [f32; 4], active: [f32; 4]) -> [f32; 4] {
    let blend = (active[3].clamp(0.0, 1.0) * 0.8).clamp(0.0, 1.0);
    mix(base, [active[0], active[1], active[2], 1.0], blend)
}
