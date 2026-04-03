use crate::render::shared::{
    PfaKeyboardProjectorConfig, ProjectedScene, SceneLayer, vertical_gradient_quad,
};

use super::super::model::PfaLayoutParams;

pub(super) fn push_keyboard_decorations(
    projector: &PfaKeyboardProjectorConfig,
    scene: &mut ProjectedScene,
    params: &PfaLayoutParams,
) {
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

    let (top, bottom) = projector.resolved_top_bar_gradient();
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
