pub const COLOR_SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexIn {
    @location(0) position0: vec3<f32>,
    @location(1) position1: vec3<f32>,
    @location(2) position2: vec3<f32>,
    @location(3) position3: vec3<f32>,
    @location(4) color0: vec4<f32>,
    @location(5) color1: vec4<f32>,
    @location(6) color2: vec4<f32>,
    @location(7) color3: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn quad_index(vertex_index: u32) -> u32 {
    switch vertex_index {
        case 0u: { return 0u; }
        case 1u: { return 1u; }
        case 2u: { return 2u; }
        case 3u: { return 0u; }
        case 4u: { return 2u; }
        default: { return 3u; }
    }
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let idx = quad_index(vertex_index);
    let positions = array<vec3<f32>, 4>(input.position0, input.position1, input.position2, input.position3);
    let colors = array<vec4<f32>, 4>(input.color0, input.color1, input.color2, input.color3);
    out.position = uniforms.mvp * vec4<f32>(positions[idx], 1.0);
    out.color = colors[idx];
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

pub const AURA_SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
@group(0) @binding(1)
var aura_texture: texture_2d<f32>;
@group(0) @binding(2)
var aura_sampler: sampler;

struct VertexIn {
    @location(0) position0: vec3<f32>,
    @location(1) position1: vec3<f32>,
    @location(2) position2: vec3<f32>,
    @location(3) position3: vec3<f32>,
    @location(4) color0: vec4<f32>,
    @location(5) color1: vec4<f32>,
    @location(6) color2: vec4<f32>,
    @location(7) color3: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

fn quad_index(vertex_index: u32) -> u32 {
    switch vertex_index {
        case 0u: { return 0u; }
        case 1u: { return 1u; }
        case 2u: { return 2u; }
        case 3u: { return 0u; }
        case 4u: { return 2u; }
        default: { return 3u; }
    }
}

fn quad_uv(corner_index: u32) -> vec2<f32> {
    switch corner_index {
        case 0u: { return vec2<f32>(0.0, 0.0); }
        case 1u: { return vec2<f32>(0.0, 1.0); }
        case 2u: { return vec2<f32>(1.0, 1.0); }
        default: { return vec2<f32>(1.0, 0.0); }
    }
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let idx = quad_index(vertex_index);
    let positions = array<vec3<f32>, 4>(input.position0, input.position1, input.position2, input.position3);
    let colors = array<vec4<f32>, 4>(input.color0, input.color1, input.color2, input.color3);
    out.position = uniforms.mvp * vec4<f32>(positions[idx], 1.0);
    out.color = colors[idx];
    out.uv = quad_uv(idx);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(aura_texture, aura_sampler, input.uv);
    return vec4<f32>(texel.rgb * input.color.rgb, texel.a * input.color.a);
}
"#;
