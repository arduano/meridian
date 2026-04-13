use super::SceneQuad;

#[expect(
    clippy::too_many_arguments,
    reason = "Gradient quad helpers take explicit corner positions and colors to match the caller's rendering data."
)]
pub(crate) fn vertical_gradient_quad(
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

pub(crate) fn quad(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4]) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth: 0.5,
        _padding: [0.0; 3],
    }
}

pub(crate) fn solid_quad(x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]) -> SceneQuad {
    quad(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [color, color, color, color],
    )
}

pub(crate) fn mod_add(color: [f32; 4], add: f32, mul: f32) -> [f32; 4] {
    [
        (color[0] * mul + add).clamp(0.0, 1.0),
        (color[1] * mul + add).clamp(0.0, 1.0),
        (color[2] * mul + add).clamp(0.0, 1.0),
        color[3],
    ]
}

pub(crate) fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

pub(crate) fn alpha_blend(top: [f32; 4], bottom: [f32; 4]) -> [f32; 4] {
    let alpha = top[3].clamp(0.0, 1.0);
    [
        top[0] * alpha + bottom[0] * (1.0 - alpha),
        top[1] * alpha + bottom[1] * (1.0 - alpha),
        top[2] * alpha + bottom[2] * (1.0 - alpha),
        1.0,
    ]
}
