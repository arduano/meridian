use crate::render::shared::{PfaKeyboardProjectorConfig, expanded_white_key_bounds};
use crate::render::shared::{ProjectedScene, SceneLayer, mod_add, quad, vertical_gradient_quad};

use super::super::model::{
    KeyColorPair, KeyPositionArrays, PfaLayoutParams, activity_pair, blend_key_pair,
};
use crate::render::shared::is_black_key;

pub(super) fn push_white_keys(
    projector: &PfaKeyboardProjectorConfig,
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    arrays: &KeyPositionArrays,
    kbfirst: usize,
    kblast: usize,
) {
    let orig_white = KeyColorPair::for_base([1.0, 1.0, 1.0, 1.0]);
    for n in kbfirst..kblast {
        if is_black_key(n as u8) {
            continue;
        }
        let (mut x1, mut x2) = (arrays.x1[n], arrays.x1[n] + arrays.width[n]);
        let width = x2 - x1;

        if projector.same_width_notes {
            (x1, x2) = expanded_white_key_bounds(x1, x2, n);
        }

        let activity = scene.key_activity(n);
        let pair = blend_key_pair(activity_pair(activity), orig_white);
        if activity.pressed {
            push_pressed_white_key(scene, params, x1, x2, pair);
        } else {
            push_unpressed_white_key(scene, params, x1, x2);
        }

        if n == 60 && projector.middle_c {
            push_middle_c_marker(scene, params, x1, x2, width, activity.pressed, pair);
        }

        push_white_separator(scene, params, arrays.width[n], x1);
    }
}

fn push_pressed_white_key(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    x1: f32,
    x2: f32,
    pair: KeyColorPair,
) {
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
}

fn push_unpressed_white_key(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    x1: f32,
    x2: f32,
) {
    scene.push_quad(
        SceneLayer::WhiteKeys,
        vertical_gradient_quad(
            x1,
            params.w_end_up_t,
            x2,
            params.top_bar_end,
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.8, 1.8, 1.8, 1.0],
            [1.8, 1.8, 1.8, 1.0],
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

fn push_middle_c_marker(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    x1: f32,
    x2: f32,
    width: f32,
    pressed: bool,
    pair: KeyColorPair,
) {
    let marker_x1 = x1 + width / 4.0;
    let marker_x2 = x2 - width / 4.0;
    let marker_y2 = if pressed {
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

fn push_white_separator(scene: &mut ProjectedScene, params: &PfaLayoutParams, width: f32, x1: f32) {
    let sep_width = ((width * params.scwidth) / 20.0).round().max(1.0);
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
