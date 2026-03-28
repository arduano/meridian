use crate::render::shared::{PfaTopColor, ProjectedScene, SceneLayer, vertical_gradient_quad};

use super::{PfaKeyboardProjector, PfaLayoutParams};

pub(super) fn push_keyboard_decorations(
    projector: &PfaKeyboardProjector,
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

    let (top, bottom) = match projector.0.top_color {
        PfaTopColor::Red => (
            [
                projector.0.top_bar_rgb[0] * 0.5,
                projector.0.top_bar_rgb[1] * 0.5,
                projector.0.top_bar_rgb[2] * 0.5,
                1.0,
            ],
            [
                projector.0.top_bar_rgb[0],
                projector.0.top_bar_rgb[1],
                projector.0.top_bar_rgb[2],
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
