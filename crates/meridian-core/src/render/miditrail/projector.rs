use glam::{Mat4, Vec3};

use crate::{
    midi::views::MIDIFileViewsUnion,
    render::{
        SceneLayout,
        shared::{MiditrailSceneConfig, ProjectedScene, alpha_blend, is_black_key},
    },
};

use super::{
    layout::MiditrailLayout,
    model::{MiditrailAuraVertex, MiditrailScene, MiditrailVertex},
};

const WHITE_KEY_LEN: f32 = 5.0;
const BLACK_KEY_LEN: f32 = 6.9;
#[derive(Clone, Copy)]
struct VisibleNote {
    key: usize,
    start: f32,
    end: f32,
    left: [f32; 4],
    right: [f32; 4],
    active: bool,
}

#[derive(Clone, Copy, Default)]
struct KeyState {
    left: [f32; 4],
    right: [f32; 4],
    press: f32,
    aura: f32,
}

pub fn project_miditrail_scene(
    config: &MiditrailSceneConfig,
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    let first_key = layout.first_key.min(layout.last_key) as usize;
    let last_key = layout.last_key.max(layout.first_key) as usize + 1;
    let key_layout = MiditrailLayout::new(first_key, last_key, config);
    let mut miditrail = scene.take_miditrail().unwrap_or_default();
    miditrail.clear();

    let notes_by_key = collect_visible_notes(config, views, &key_layout, layout.view_range as f32);
    let key_order = key_render_order(config, &key_layout);

    let mut key_state = vec![KeyState::default(); 256];
    for notes in &notes_by_key {
        for note in notes {
            scene.visible_notes += 1;
            scene.note_quads += 1 + usize::from(config.box_notes) * 2;
            if note.active {
                let state = &mut key_state[note.key];
                state.left = alpha_blend(note.left, state.left);
                state.right = alpha_blend(note.right, state.right);
                state.press = state.press.max(config.note_down_speed.clamp(0.0, 1.0));
                state.aura = state.aura.max(0.5);
            }
        }
    }

    for &key in &key_order {
        let notes = &notes_by_key[key - key_layout.first_key];
        if config.box_notes {
            for note in notes.iter().rev() {
                emit_note_cap(
                    &mut miditrail,
                    &key_layout,
                    config,
                    layout.view_range as f32,
                    *note,
                );
            }
            for note in notes {
                emit_note_side(
                    &mut miditrail,
                    &key_layout,
                    config,
                    layout.view_range as f32,
                    *note,
                );
            }
        }
        for note in notes {
            emit_note_front(
                &mut miditrail,
                &key_layout,
                config,
                layout.view_range as f32,
                *note,
            );
        }
    }

    if config.show_keyboard {
        emit_keyboard(
            &mut miditrail,
            &key_layout,
            config,
            &key_state,
            &mut scene.active_keys,
            &mut scene.keyboard_quads,
        );
    }
    if config.aura_enabled {
        emit_aura(&mut miditrail, &key_layout, config, &key_state);
    }

    scene.set_miditrail(miditrail);
}

fn collect_visible_notes(
    config: &MiditrailSceneConfig,
    views: &MIDIFileViewsUnion<'_>,
    key_layout: &MiditrailLayout,
    view_range: f32,
) -> Vec<Vec<VisibleNote>> {
    let mut notes = vec![Vec::new(); key_layout.last_key_exclusive - key_layout.first_key];
    let render_start = -view_range * config.viewback;
    for key in key_layout.first_key..key_layout.last_key_exclusive {
        let column = views.get_column(key);
        column.for_each_displaced_note(|note| {
            let start = note.start;
            let mut end = start + note.len;
            if end < render_start || start >= view_range {
                return;
            }
            let mut clamped_start = start.max(render_start);
            if config.eat_notes {
                clamped_start = clamped_start.max(0.0);
                end = end.max(0.0);
            }
            notes[key - key_layout.first_key].push(VisibleNote {
                key,
                start: clamped_start,
                end: end.min(view_range),
                left: note.color.left.to_rgba(1.0),
                right: note.color.right.to_rgba(1.0),
                active: note.start <= 0.0 && end > 0.0,
            });
        });
    }
    for key_notes in &mut notes {
        key_notes.sort_by(|a, b| {
            a.start
                .total_cmp(&b.start)
                .then_with(|| a.end.total_cmp(&b.end))
        });
    }
    notes
}

fn key_render_order(config: &MiditrailSceneConfig, key_layout: &MiditrailLayout) -> Vec<usize> {
    let model = if config.vertical_notes {
        Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Mat4::IDENTITY
    };
    let view = Mat4::from_rotation_x(config.cam_ang)
        * Mat4::from_rotation_y(config.cam_rot)
        * Mat4::from_rotation_z(config.cam_spin)
        * Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0))
        * Mat4::from_translation(Vec3::new(
            config.view_pan,
            -config.view_height,
            -config.view_offset,
        ))
        * model;
    let mut keys: Vec<_> = (key_layout.first_key..key_layout.last_key_exclusive).collect();
    keys.sort_by(|&a, &b| {
        let av = view.transform_point3(Vec3::new(
            key_layout.key_x1(a) + key_layout.key_width(a) * 0.5,
            0.0,
            0.0,
        ));
        let bv = view.transform_point3(Vec3::new(
            key_layout.key_x1(b) + key_layout.key_width(b) * 0.5,
            0.0,
            0.0,
        ));
        let ad = av.length_squared();
        let bd = bv.length_squared();
        bd.total_cmp(&ad)
    });
    keys
}

fn emit_note_side(
    scene: &mut MiditrailScene,
    key_layout: &MiditrailLayout,
    config: &MiditrailSceneConfig,
    view_range: f32,
    note: VisibleNote,
) {
    let base_x1 = key_layout.key_x1(note.key);
    let width = key_layout.key_width(note.key);
    let base_x2 = base_x1 + width;
    let z1 = note.end * (config.viewdist / view_range.max(0.0001));
    let z2 = note.start * (config.viewdist / view_range.max(0.0001));

    let factor = active_factor(note, config);
    let mut side_x = base_x1;
    if side_x < -config.view_pan {
        side_x += width;
    }
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
    push_quad(
        &mut scene.note_vertices,
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

fn emit_note_cap(
    scene: &mut MiditrailScene,
    key_layout: &MiditrailLayout,
    config: &MiditrailSceneConfig,
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
    push_quad(
        &mut scene.note_vertices,
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
    scene: &mut MiditrailScene,
    key_layout: &MiditrailLayout,
    config: &MiditrailSceneConfig,
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
    push_quad(
        &mut scene.note_vertices,
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

fn active_factor(note: VisibleNote, config: &MiditrailSceneConfig) -> f32 {
    if !note.active {
        return 0.0;
    }
    config.note_down_speed.clamp(0.0, 1.0) * 0.5
}

fn cap_or_front_shape(
    base_x1: f32,
    base_x2: f32,
    width: f32,
    note: VisibleNote,
    config: &MiditrailSceneConfig,
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
    scene: &mut MiditrailScene,
    key_layout: &MiditrailLayout,
    config: &MiditrailSceneConfig,
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
        let blend = 0.8 * key_state[key].left[3].max(key_state[key].right[3]);
        let left = mix(base_left, key_state[key].left, blend);
        let right = mix(base_right, key_state[key].right, blend);
        let press_y = if config.tilt_keys {
            0.0
        } else if is_black_key(key as u8) {
            -key_state[key].press / 1.2
        } else {
            -key_state[key].press / 2.0
        };

        if is_black_key(key as u8) {
            let x1 = key_layout.key_x1(key);
            let x2 = x1 + key_layout.key_width(key);
            let top_y = 1.2 + press_y;
            push_box(
                &mut scene.black_key_vertices,
                x1,
                x2,
                0.0 + press_y,
                top_y,
                -BLACK_KEY_LEN,
                0.0,
                left,
                right,
            );
            *keyboard_quads += 6;
        } else {
            let (x1, x2) = if config.same_width_notes {
                key_layout.expanded_white_key_span(key)
            } else {
                let x1 = key_layout.key_x1(key);
                (x1, x1 + key_layout.key_width(key))
            };
            let back_scale = if config.same_width_notes {
                key_layout.key_width(key) * 2.0
            } else {
                x2 - x1
            };
            push_box(
                &mut scene.white_key_vertices,
                x1,
                x2,
                -0.3 + press_y,
                0.6 + press_y,
                -back_scale * WHITE_KEY_LEN * 0.95,
                0.0,
                left,
                right,
            );
            *keyboard_quads += 6;
        }
    }
}

fn emit_aura(
    scene: &mut MiditrailScene,
    key_layout: &MiditrailLayout,
    config: &MiditrailSceneConfig,
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

fn push_box(
    out: &mut Vec<MiditrailVertex>,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    z1: f32,
    z2: f32,
    left: [f32; 4],
    right: [f32; 4],
) {
    push_quad(
        out,
        [x1, y1, z1],
        [x1, y2, z1],
        [x2, y2, z1],
        [x2, y1, z1],
        left,
        left,
        right,
        right,
    );
    push_quad(
        out,
        [x1, y2, z1],
        [x1, y2, z2],
        [x2, y2, z2],
        [x2, y2, z1],
        dim(left, 1.0),
        left,
        right,
        dim(right, 1.0),
    );
    push_quad(
        out,
        [x1, y1, z2],
        [x1, y1, z1],
        [x1, y2, z1],
        [x1, y2, z2],
        dim(left, 0.8),
        dim(left, 0.8),
        left,
        dim(left, 0.8),
    );
    push_quad(
        out,
        [x2, y1, z2],
        [x2, y1, z1],
        [x2, y2, z1],
        [x2, y2, z2],
        dim(right, 0.8),
        dim(right, 0.8),
        right,
        dim(right, 0.8),
    );
    push_quad(
        out,
        [x1, y1, z2],
        [x1, y2, z2],
        [x2, y2, z2],
        [x2, y1, z2],
        dim(left, 0.9),
        left,
        right,
        dim(right, 0.9),
    );
    push_quad(
        out,
        [x1, y1, z1],
        [x1, y1, z2],
        [x2, y1, z2],
        [x2, y1, z1],
        dim(left, 0.8),
        dim(left, 0.8),
        dim(right, 0.8),
        dim(right, 0.8),
    );
}

fn push_quad(
    out: &mut Vec<MiditrailVertex>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    ca: [f32; 4],
    cb: [f32; 4],
    cc: [f32; 4],
    cd: [f32; 4],
) {
    out.extend_from_slice(&[
        MiditrailVertex {
            position: a,
            color: ca,
        },
        MiditrailVertex {
            position: b,
            color: cb,
        },
        MiditrailVertex {
            position: c,
            color: cc,
        },
        MiditrailVertex {
            position: a,
            color: ca,
        },
        MiditrailVertex {
            position: c,
            color: cc,
        },
        MiditrailVertex {
            position: d,
            color: cd,
        },
    ]);
}

fn push_aura_quad(
    scene: &mut MiditrailScene,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    left: [f32; 4],
    right: [f32; 4],
) {
    scene.aura_vertices.extend_from_slice(&[
        MiditrailAuraVertex {
            position: a,
            color: left,
            uv: [0.0, 0.0],
            _padding: [0.0; 2],
        },
        MiditrailAuraVertex {
            position: b,
            color: left,
            uv: [0.0, 1.0],
            _padding: [0.0; 2],
        },
        MiditrailAuraVertex {
            position: c,
            color: right,
            uv: [1.0, 1.0],
            _padding: [0.0; 2],
        },
        MiditrailAuraVertex {
            position: a,
            color: left,
            uv: [0.0, 0.0],
            _padding: [0.0; 2],
        },
        MiditrailAuraVertex {
            position: c,
            color: right,
            uv: [1.0, 1.0],
            _padding: [0.0; 2],
        },
        MiditrailAuraVertex {
            position: d,
            color: right,
            uv: [1.0, 0.0],
            _padding: [0.0; 2],
        },
    ]);
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

fn dim(color: [f32; 4], factor: f32) -> [f32; 4] {
    [
        (color[0] * factor).clamp(0.0, 1.0),
        (color[1] * factor).clamp(0.0, 1.0),
        (color[2] * factor).clamp(0.0, 1.0),
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
