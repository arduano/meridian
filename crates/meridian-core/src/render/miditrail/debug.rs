use glam::{Mat4, Vec3, Vec4};
use serde::Serialize;

use crate::render::shared::{MiditrailSceneConfig, is_black_key};

use super::layout::MiditrailLayout;

const KEY_X_SQUEEZE: f32 = 0.95;
const WHITE_KEY_Y_DROP: f32 = 0.3;
const BLACK_KEY_Y_LIFT: f32 = 1.2;

#[derive(Debug, Clone, Serialize)]
pub struct MiditrailGeometryDump {
    pub same_width_notes: bool,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub keys: Vec<MiditrailKeyGeometry>,
    pub overlaps: Vec<MiditrailOverlap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiditrailKeyGeometry {
    pub key: u8,
    pub is_black: bool,
    pub note_x1: f32,
    pub note_x2: f32,
    pub note_center: f32,
    pub note_center_ndc_x: f32,
    pub keyboard_x1: f32,
    pub keyboard_x2: f32,
    pub keyboard_center: f32,
    pub keyboard_anchor_ndc_x: f32,
    pub width_scale_x: f32,
    pub width_scale_yz: f32,
    pub slot_left: Option<f32>,
    pub slot_right: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiditrailOverlap {
    pub relation: &'static str,
    pub left_key: u8,
    pub right_key: u8,
    pub overlap_x1: f32,
    pub overlap_x2: f32,
    pub overlap_width: f32,
}

pub fn dump_miditrail_geometry(
    config: &MiditrailSceneConfig,
    first_key: u8,
    last_key: u8,
    viewport_width: u32,
    viewport_height: u32,
) -> MiditrailGeometryDump {
    let first = first_key.min(last_key) as usize;
    let last_exclusive = first_key.max(last_key) as usize + 1;
    let layout = MiditrailLayout::new(first, last_exclusive, config);
    let mvp = build_mvp(config, viewport_width as f32 / viewport_height.max(1) as f32);
    let mut keys = Vec::with_capacity(last_exclusive.saturating_sub(first));
    for key in first..last_exclusive {
        let note_x1 = layout.key_x1(key);
        let note_x2 = note_x1 + layout.key_width(key);
        let note_center = (note_x1 + note_x2) * 0.5;
        let note_center_ndc_x = project_ndc_x(mvp, [note_center, 0.0, 0.0]);
        let is_black = is_black_key(key as u8);
        let (keyboard_x1, keyboard_x2, keyboard_anchor_ndc_x, scale_x, scale_yz, slot_left, slot_right) =
            if is_black {
                let base_x = layout.key_x1(key);
                let width = layout.key_width(key);
                let scale_x = width;
                let scale_yz = width;
                let anchor_world = [base_x + scale_x * 0.5, BLACK_KEY_Y_LIFT * (width / 1.2), 0.0];
                (
                    base_x + scale_x * (1.0 - KEY_X_SQUEEZE) * 0.5,
                    base_x + scale_x * (1.0 + KEY_X_SQUEEZE) * 0.5,
                    project_ndc_x(mvp, anchor_world),
                    scale_x,
                    scale_yz,
                    None,
                    None,
                )
            } else {
                let (base_x, base_x2) = if config.same_width_notes {
                    layout.expanded_white_key_span(key)
                } else {
                    let x1 = layout.key_x1(key);
                    (x1, x1 + layout.key_width(key))
                };
                let width = base_x2 - base_x;
                let scale_x = width;
                let scale_yz = if config.same_width_notes {
                    layout.key_width(layout.first_key) * 2.0
                } else {
                    width
                };
                let (offset_left, offset_right) = white_key_offsets(&layout, key, white_pitch_index(key as u8));
                let squeezed_x1 = base_x + scale_x * (1.0 - KEY_X_SQUEEZE) * 0.5;
                let squeezed_x2 = base_x + scale_x * (1.0 + KEY_X_SQUEEZE) * 0.5;
                let slot_left = base_x + ((offset_left - 0.5) * KEY_X_SQUEEZE + 0.5) * scale_x;
                let slot_right = base_x + ((offset_right - 0.5) * KEY_X_SQUEEZE + 0.5) * scale_x;
                let slot_center = base_x + ((((offset_left + offset_right) * 0.5) - 0.5) * KEY_X_SQUEEZE + 0.5) * scale_x;
                let anchor_world = [slot_center, (0.5 - WHITE_KEY_Y_DROP) * scale_yz, 0.0];
                (
                    squeezed_x1,
                    squeezed_x2,
                    project_ndc_x(mvp, anchor_world),
                    scale_x,
                    scale_yz,
                    Some(slot_left),
                    Some(slot_right),
                )
            };
        keys.push(MiditrailKeyGeometry {
            key: key as u8,
            is_black,
            note_x1,
            note_x2,
            note_center,
            note_center_ndc_x,
            keyboard_x1,
            keyboard_x2,
            keyboard_center: (keyboard_x1 + keyboard_x2) * 0.5,
            keyboard_anchor_ndc_x,
            width_scale_x: scale_x,
            width_scale_yz: scale_yz,
            slot_left,
            slot_right,
        });
    }
    let overlaps = compute_overlaps(&keys);
    MiditrailGeometryDump {
        same_width_notes: config.same_width_notes,
        first_key,
        last_key,
        viewport_width,
        viewport_height,
        keys,
        overlaps,
    }
}

fn build_mvp(config: &MiditrailSceneConfig, aspect: f32) -> Mat4 {
    let mut model = Mat4::IDENTITY;
    if config.vertical_notes {
        model *= Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    }
    Mat4::perspective_rh_gl(config.fov, aspect.max(0.001), 0.01, 400.0)
        * Mat4::from_rotation_x(config.cam_ang)
        * Mat4::from_rotation_y(config.cam_rot)
        * Mat4::from_rotation_z(config.cam_spin)
        * Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0))
        * Mat4::from_translation(Vec3::new(
            config.view_pan,
            -config.view_height,
            config.view_offset,
        ))
        * model
}

fn project_ndc_x(mvp: Mat4, world: [f32; 3]) -> f32 {
    let clip = mvp * Vec4::new(world[0], world[1], world[2], 1.0);
    if clip.w.abs() <= 1.0e-6 {
        0.0
    } else {
        clip.x / clip.w
    }
}

fn compute_overlaps(keys: &[MiditrailKeyGeometry]) -> Vec<MiditrailOverlap> {
    let mut overlaps = Vec::new();
    for window in keys.windows(2) {
        let left = &window[0];
        let right = &window[1];
        let overlap_x1 = left.keyboard_x1.max(right.keyboard_x1);
        let overlap_x2 = left.keyboard_x2.min(right.keyboard_x2);
        let overlap_width = (overlap_x2 - overlap_x1).max(0.0);
        if overlap_width <= 0.0 {
            continue;
        }
        let relation = match (left.is_black, right.is_black) {
            (false, false) => "white_white",
            (false, true) | (true, false) => "white_black",
            (true, true) => "black_black",
        };
        overlaps.push(MiditrailOverlap {
            relation,
            left_key: left.key,
            right_key: right.key,
            overlap_x1,
            overlap_x2,
            overlap_width,
        });
    }
    overlaps
}

fn white_key_offsets(key_layout: &MiditrailLayout, key: usize, pitch: usize) -> (f32, f32) {
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
