use crate::render::shared::{ProjectedScene, SceneLayer, mod_add, quad};

use super::super::model::{
    KeyColorPair, KeyPositionArrays, PfaLayoutParams, activity_pair, blend_key_pair,
};
use crate::render::shared::is_black_key;

pub(super) fn push_black_keys(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    arrays: &KeyPositionArrays,
    kbfirst: usize,
    kblast: usize,
) {
    let orig_black = KeyColorPair::for_base([0.0, 0.0, 0.0, 1.0]);
    for n in kbfirst..kblast {
        if !is_black_key(n as u8) {
            continue;
        }

        let ox1 = arrays.x1[n];
        let width = arrays.width[n];
        let ox2 = ox1 + width;
        let ix1 = ox1 + width / 8.0;
        let ix2 = ox2 - width / 8.0;
        let activity = scene.key_activity(n);
        let pair = blend_key_pair(activity_pair(activity), orig_black);
        let mid = [
            (pair.left[0] + pair.right[0]) / 2.0,
            (pair.left[1] + pair.right[1]) / 2.0,
            (pair.left[2] + pair.right[2]) / 2.0,
            1.0,
        ];

        let top = if activity.pressed {
            params.b_key_down_t
        } else {
            params.b_key_up_t
        };
        let bottom = if activity.pressed {
            params.b_key_down_b
        } else {
            params.b_key_up_b
        };

        if activity.pressed {
            push_pressed_black_key(scene, params, ox1, ox2, ix1, ix2, top, bottom, pair, mid);
        } else {
            push_unpressed_black_key(scene, params, ox1, ox2, ix1, ix2, top, bottom, pair);
        }
        scene.keyboard_quads += 6;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Pressed black keys are emitted from precomputed geometry scalars for speed and readability."
)]
fn push_pressed_black_key(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    ox1: f32,
    ox2: f32,
    ix1: f32,
    ix2: f32,
    top: f32,
    bottom: f32,
    pair: KeyColorPair,
    mid: [f32; 4],
) {
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
}

#[expect(
    clippy::too_many_arguments,
    reason = "Unpressed black keys share the same explicit geometry inputs as the pressed path."
)]
fn push_unpressed_black_key(
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
    ox1: f32,
    ox2: f32,
    ix1: f32,
    ix2: f32,
    top: f32,
    bottom: f32,
    pair: KeyColorPair,
) {
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
