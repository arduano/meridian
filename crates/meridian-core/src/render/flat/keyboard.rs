use super::{
    super::{
        SceneLayout,
        shared::{
            ProjectedScene, SceneLayer, expanded_white_key_bounds, is_black_key, linear_key_span,
            mix, normalized_key_range, solid_quad,
        },
    },
    FlatKeyState,
};

pub(crate) fn project_flat_keyboard(
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    let (first_key, last_key_exclusive) = normalized_key_range(layout.first_key, layout.last_key);
    let (first_key, last_key, key_width) = linear_key_span(first_key, last_key_exclusive);
    let mut key_states = Vec::with_capacity(last_key.saturating_sub(first_key) + 1);

    for key in first_key..=last_key {
        let x1 = (key - first_key) as f32 * key_width;
        let x2 = x1 + key_width;
        key_states.push(FlatKeyState {
            x1,
            x2,
            is_black: is_black_key(key as u8),
            activity: scene.key_activity(key),
        });
    }

    scene.push_quad(
        SceneLayer::WhiteKeys,
        solid_quad(0.0, 0.0, 1.0, piano_height, [0.96, 0.96, 0.96, 1.0]),
    );
    scene.keyboard_quads += 1;

    for (offset, key_state) in key_states.iter().enumerate() {
        if key_state.is_black {
            continue;
        }

        let key = first_key + offset;
        let (x1, x2) = expanded_white_key_bounds(key_state.x1, key_state.x2, key);
        let pressed = key_state.activity.left;
        let color = if key_state.activity.pressed {
            mix([1.0, 1.0, 1.0, 1.0], pressed, 0.96)
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };

        scene.push_quad(
            SceneLayer::WhiteKeys,
            solid_quad(x1.max(0.0), 0.0, x2.min(1.0), piano_height, color),
        );
        scene.keyboard_quads += 1;
    }

    for key_state in &key_states {
        if !key_state.is_black {
            continue;
        }

        let base = [0.02, 0.02, 0.02, 1.0];
        let color = if key_state.activity.pressed {
            mix(base, key_state.activity.left, 0.96)
        } else {
            base
        };
        scene.push_quad(
            SceneLayer::BlackKeys,
            solid_quad(
                key_state.x1,
                piano_height * 0.37,
                key_state.x2,
                piano_height,
                color,
            ),
        );
        scene.keyboard_quads += 1;
    }
}
