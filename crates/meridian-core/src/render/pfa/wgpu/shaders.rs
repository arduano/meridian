pub(super) const FLAT_SHADER: &str = r#"
struct QuadInstance {
    pos0: vec2<f32>,
    pos1: vec2<f32>,
    pos2: vec2<f32>,
    pos3: vec2<f32>,
    color0: vec4<f32>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    color3: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn bilerp2(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>, uv: vec2<f32>) -> vec2<f32> {
    let bottom = mix(a, b, uv.x);
    let top = mix(d, c, uv.x);
    return mix(bottom, top, uv.y);
}

fn bilerp4(a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, d: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let bottom = mix(a, b, uv.x);
    let top = mix(d, c, uv.x);
    return mix(bottom, top, uv.y);
}

@vertex
fn vs_main(
    @location(0) unit_position: vec2<f32>,
    @location(1) pos0: vec2<f32>,
    @location(2) pos1: vec2<f32>,
    @location(3) pos2: vec2<f32>,
    @location(4) pos3: vec2<f32>,
    @location(5) color0: vec4<f32>,
    @location(6) color1: vec4<f32>,
    @location(7) color2: vec4<f32>,
    @location(8) color3: vec4<f32>,
    @location(9) depth: f32,
) -> VsOut {
    let uv = unit_position;
    let pos = bilerp2(pos0, pos1, pos2, pos3, uv);
    var out: VsOut;
    out.position = vec4<f32>(pos.x * 2.0 - 1.0, pos.y * 2.0 - 1.0, depth, 1.0);
    out.color = bilerp4(color0, color1, color2, color3, uv);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub(super) const NOTE_SHADER: &str = r#"
struct NoteParams {
    piano_height: f32,
    note_pos_factor: f32,
    pad_x: f32,
    pad_y: f32,
};

struct KeyPositions {
    values: array<vec4<f32>, 256>,
};

@group(0) @binding(0)
var<uniform> note_params: NoteParams;

@group(0) @binding(1)
var<uniform> key_positions: KeyPositions;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: vec2<f32>,
    @location(3) pad: vec2<f32>,
};

fn mod_add(color: vec4<f32>, add: f32, mul: f32) -> vec4<f32> {
    return vec4<f32>(
        clamp(color.r * mul + add, 0.0, 1.0),
        clamp(color.g * mul + add, 0.0, 1.0),
        clamp(color.b * mul + add, 0.0, 1.0),
        color.a,
    );
}

@vertex
fn vs_main(
    @location(0) unit_position: vec2<f32>,
    @location(1) key: u32,
    @location(2) start: f32,
    @location(3) end: f32,
    @location(4) color: vec4<f32>,
) -> VsOut {
    let key_pos = key_positions.values[key];
    let x1 = key_pos.x;
    let x2 = key_pos.y;
    let y1 = note_params.piano_height + start * note_params.note_pos_factor;
    let y2 = note_params.piano_height + end * note_params.note_pos_factor;

    var out: VsOut;
    let pos = vec2<f32>(mix(x1, x2, unit_position.x), mix(y1, y2, unit_position.y));
    out.position = vec4<f32>(pos.x * 2.0 - 1.0, pos.y * 2.0 - 1.0, 0.5, 1.0);
    out.uv = unit_position;
    out.color = color;
    out.size = vec2<f32>(max(x2 - x1, 1e-6), max(y2 - y1, 1e-6));
    out.pad = vec2<f32>(note_params.pad_x, note_params.pad_y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let border_u = clamp(in.pad.x / in.size.x, 0.0, 0.49);
    let border_v = clamp(in.pad.y / in.size.y, 0.0, 0.49);
    let has_inner = in.size.x > in.pad.x * 2.0 && in.size.y > in.pad.y * 2.0;
    let inside_inner =
        has_inner &&
        in.uv.x >= border_u &&
        in.uv.x <= (1.0 - border_u) &&
        in.uv.y >= border_v &&
        in.uv.y <= (1.0 - border_v);

    if inside_inner {
        let inner_u = clamp((in.uv.x - border_u) / max(1.0 - border_u * 2.0, 1e-6), 0.0, 1.0);
        let left = mod_add(in.color, 0.18, 1.0);
        let right = mod_add(in.color, 0.0, 0.55);
        return mix(left, right, inner_u);
    }

    let border_left = mod_add(in.color, 0.0, 0.3);
    let border_right = mod_add(in.color, 0.0, 0.12);
    return mix(border_left, border_right, in.uv.x);
}
"#;
