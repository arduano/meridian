use std::borrow::Cow;
use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::wgpu_28::wgpu;
use wgpu::util::DeviceExt;

const VIEWPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

const SHADER: &str = r#"
struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    let clip = positions[vertex_index];
    var out: VsOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = clip * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let centered = in.uv - vec2<f32>(0.5, 0.5);
    let dist = length(centered * vec2<f32>(uniforms.resolution.x / max(uniforms.resolution.y, 1.0), 1.0));
    let wave = 0.5 + 0.5 * sin(dist * 18.0 - uniforms.time * 2.1);
    let sweep = 0.5 + 0.5 * sin((in.uv.x * 10.0) + uniforms.time * 1.7);
    let glow = max(0.0, 1.0 - dist * 1.5);

    let base = vec3<f32>(0.04, 0.08, 0.12);
    let color = base
        + vec3<f32>(0.05, 0.32, 0.70) * sweep
        + vec3<f32>(0.85, 0.45, 0.16) * wave * glow;

    return vec4<f32>(color, 1.0);
}
"#;

slint::slint! {
    component MicroButton inherits Rectangle {
        in property <string> label;
        in property <color> fill: #2a3440;
        in property <color> stroke: #4d6177;
        in property <color> ink: white;
        callback pressed;

        min-width: 34px;
        min-height: 30px;
        border-radius: 6px;
        background: touch.pressed ? stroke : fill;
        border-width: 1px;
        border-color: stroke;

        Text {
            text: parent.label;
            color: parent.ink;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-weight: 700;
        }

        touch := TouchArea {
            clicked => { root.pressed(); }
        }
    }

    component Chip inherits Rectangle {
        in property <string> label;
        in property <bool> active;
        in property <color> fill;
        in property <color> active-fill;
        in property <color> stroke;
        in property <color> ink;
        callback pressed;

        min-width: 74px;
        min-height: 28px;
        border-radius: 999px;
        background: active ? active-fill : fill;
        border-width: 1px;
        border-color: stroke;

        Text {
            text: parent.label;
            color: parent.ink;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-weight: active ? 700 : 500;
            font-size: 13px;
        }

        touch := TouchArea {
            clicked => { root.pressed(); }
        }
    }

    component ThemeCard inherits Rectangle {
        in property <string> theme-name;
        in property <string> subtitle;
        in property <string> mode-label;
        in property <int> exposure;
        in property <int> speed;
        in property <bool> bloom;
        in property <color> shell;
        in property <color> shell-2;
        in property <color> stroke;
        in property <color> accent;
        in property <color> accent-2;
        in property <color> ink;
        in property <color> muted;
        in property <int> radius: 14;
        in property <int> title-size: 22;
        in property <int> body-size: 13;
        in property <bool> heavy-border: false;
        in property <bool> all-caps: false;
        in property <bool> skinny: false;
        callback set-mode(int);
        callback bump-exposure(int);
        callback bump-speed(int);
        callback toggle-bloom();
        callback reset-all();

        border-radius: radius * 1px;
        background: shell;
        border-width: heavy-border ? 3px : 1px;
        border-color: stroke;
        clip: true;
        min-height: 262px;

        Rectangle {
            height: 8px;
            width: parent.width;
            background: accent;
        }

        VerticalLayout {
            padding: skinny ? 10px : 14px;
            spacing: skinny ? 8px : 10px;

            Rectangle {
                background: shell-2;
                border-radius: (radius - 4) * 1px;
                border-width: 1px;
                border-color: stroke;
                height: 56px;

                VerticalLayout {
                    padding: 10px;
                    spacing: 2px;

                    Text {
                        text: theme-name;
                        color: ink;
                        font-size: title-size * 1px;
                        font-weight: 800;
                    }

                    Text {
                        text: subtitle;
                        color: muted;
                        font-size: body-size * 1px;
                    }
                }
            }

            Rectangle {
                background: shell-2;
                border-radius: (radius - 4) * 1px;
                border-width: 1px;
                border-color: stroke;

                VerticalLayout {
                    padding: 10px;
                    spacing: 8px;

                    HorizontalLayout {
                        alignment: start;
                        Text { text: all-caps ? "CAMERA MODE" : "Camera mode"; color: muted; font-size: body-size * 1px; }
                        Rectangle { horizontal-stretch: 1; }
                        Text { text: mode-label; color: accent; font-weight: 700; font-size: body-size * 1px; }
                    }

                    HorizontalLayout {
                        spacing: 6px;
                        Chip { label: "Orbit"; active: mode-label == "Orbit"; fill: shell; active-fill: accent; stroke: stroke; ink: mode-label == "Orbit" ? shell : ink; pressed => { root.set-mode(0); } }
                        Chip { label: "Pan"; active: mode-label == "Pan"; fill: shell; active-fill: accent-2; stroke: stroke; ink: mode-label == "Pan" ? shell : ink; pressed => { root.set-mode(1); } }
                        Chip { label: "Inspect"; active: mode-label == "Inspect"; fill: shell; active-fill: ink; stroke: stroke; ink: mode-label == "Inspect" ? shell : ink; pressed => { root.set-mode(2); } }
                    }
                }
            }

            HorizontalLayout {
                spacing: 8px;

                Rectangle {
                    horizontal-stretch: 1;
                    background: shell-2;
                    border-radius: (radius - 4) * 1px;
                    border-width: 1px;
                    border-color: stroke;

                    VerticalLayout {
                        padding: 10px;
                        spacing: 6px;
                        Text { text: all-caps ? "EXPOSURE" : "Exposure"; color: muted; font-size: body-size * 1px; }
                        HorizontalLayout {
                            spacing: 6px;
                            MicroButton { label: "−"; fill: shell; stroke: stroke; ink: ink; pressed => { root.bump-exposure(-5); } }
                            Rectangle {
                                horizontal-stretch: 1;
                                border-radius: 8px;
                                background: shell;
                                border-width: 1px;
                                border-color: stroke;
                                Text { text: exposure + "%"; color: ink; horizontal-alignment: center; vertical-alignment: center; font-weight: 700; }
                            }
                            MicroButton { label: "+"; fill: shell; stroke: stroke; ink: ink; pressed => { root.bump-exposure(5); } }
                        }
                    }
                }

                Rectangle {
                    horizontal-stretch: 1;
                    background: shell-2;
                    border-radius: (radius - 4) * 1px;
                    border-width: 1px;
                    border-color: stroke;

                    VerticalLayout {
                        padding: 10px;
                        spacing: 6px;
                        Text { text: all-caps ? "SPEED" : "Speed"; color: muted; font-size: body-size * 1px; }
                        HorizontalLayout {
                            spacing: 6px;
                            MicroButton { label: "−"; fill: shell; stroke: stroke; ink: ink; pressed => { root.bump-speed(-1); } }
                            Rectangle {
                                horizontal-stretch: 1;
                                border-radius: 8px;
                                background: shell;
                                border-width: 1px;
                                border-color: stroke;
                                Text { text: speed + "x"; color: ink; horizontal-alignment: center; vertical-alignment: center; font-weight: 700; }
                            }
                            MicroButton { label: "+"; fill: shell; stroke: stroke; ink: ink; pressed => { root.bump-speed(1); } }
                        }
                    }
                }
            }

            HorizontalLayout {
                spacing: 8px;

                Rectangle {
                    horizontal-stretch: 1;
                    background: shell-2;
                    border-radius: (radius - 4) * 1px;
                    border-width: 1px;
                    border-color: stroke;

                    HorizontalLayout {
                        padding: 10px;
                        spacing: 8px;
                        Text { text: all-caps ? "BLOOM" : "Bloom"; color: muted; font-size: body-size * 1px; }
                        Rectangle { horizontal-stretch: 1; }
                        Chip { label: bloom ? "On" : "Off"; active: bloom; fill: shell; active-fill: accent; stroke: stroke; ink: bloom ? shell : ink; pressed => { root.toggle-bloom(); } }
                    }
                }

                MicroButton {
                    label: all-caps ? "RESET" : "Reset";
                    min-width: 86px;
                    fill: accent;
                    stroke: accent;
                    ink: shell;
                    pressed => { root.reset-all(); }
                }
            }
        }
    }

    export component App inherits Window {
        in-out property <image> viewport-image;
        in-out property <string> status-text: "Waiting for the wgpu renderer";
        in-out property <int> mode-index: 0;
        in-out property <int> exposure: 65;
        in-out property <int> speed: 4;
        in-out property <bool> bloom: true;
        in-out property <int> selected-demo: 0;
        out property <int> viewport-px-width: viewport-box.width / 1px;
        out property <int> viewport-px-height: viewport-box.height / 1px;
        private property <string> mode-label: mode-index == 0 ? "Orbit" : mode-index == 1 ? "Pan" : "Inspect";

        callback set-mode(int);
        callback bump-exposure(int);
        callback bump-speed(int);
        callback toggle-bloom();
        callback reset-all();
        callback select-demo(int);

        set-mode(mode) => {
            if (mode < 0) {
                root.mode-index = 0;
            } else if (mode > 2) {
                root.mode-index = 2;
            } else {
                root.mode-index = mode;
            }
        }

        bump-exposure(delta) => {
            let next = root.exposure + delta;
            if (next < 0) {
                root.exposure = 0;
            } else if (next > 100) {
                root.exposure = 100;
            } else {
                root.exposure = next;
            }
        }

        bump-speed(delta) => {
            let next = root.speed + delta;
            if (next < 1) {
                root.speed = 1;
            } else if (next > 9) {
                root.speed = 9;
            } else {
                root.speed = next;
            }
        }

        toggle-bloom() => { root.bloom = !root.bloom; }
        reset-all() => {
            root.mode-index = 0;
            root.exposure = 65;
            root.speed = 4;
            root.bloom = true;
        }
        select-demo(index) => {
            if (index < 0) {
                root.selected-demo = 0;
            } else if (index > 7) {
                root.selected-demo = 7;
            } else {
                root.selected-demo = index;
            }
        }

        title: "Slint style gallery + embedded wgpu viewport";
        preferred-width: 1640px;
        preferred-height: 1320px;
        background: #0d1014;

        HorizontalLayout {
            padding: 16px;
            spacing: 16px;

            Rectangle {
                width: 980px;
                border-radius: 18px;
                background: #11161c;
                border-width: 1px;
                border-color: #283240;

                VerticalLayout {
                    padding: 16px;
                    spacing: 12px;

                    Rectangle {
                        height: 94px;
                        border-radius: 14px;
                        background: #151d25;
                        border-width: 1px;
                        border-color: #2b3949;

                        VerticalLayout {
                            padding: 14px;
                            spacing: 6px;

                            Text {
                                text: "Same control form, radically different Slint treatments";
                                color: #eef4fb;
                                font-size: 27px;
                                font-weight: 800;
                            }

                            Text {
                                text: "Every panel below is the same viewport-control form bound to shared state. The point is not color-swapping — it is proving how far you can push shape, density, hierarchy, framing, and fake-elevation while staying in vanilla Slint.";
                                color: #9fb0c3;
                                wrap: word-wrap;
                            }
                        }
                    }

                    Rectangle {
                        border-radius: 16px;
                        background: #121821;
                        border-width: 1px;
                        border-color: #2a3645;

                        VerticalLayout {
                            padding: 12px;
                            spacing: 12px;

                            HorizontalLayout {
                                spacing: 8px;
                                Chip { label: "Brutalist"; active: root.selected-demo == 0; fill: #1a2230; active-fill: #ff5e00; stroke: #334155; ink: root.selected-demo == 0 ? white : #c8d4e0; pressed => { root.select-demo(0); } }
                                Chip { label: "Terminal"; active: root.selected-demo == 1; fill: #1a2230; active-fill: #49ff8d; stroke: #334155; ink: root.selected-demo == 1 ? #08140c : #c8d4e0; pressed => { root.select-demo(1); } }
                                Chip { label: "Cute"; active: root.selected-demo == 2; fill: #1a2230; active-fill: #ff6ea8; stroke: #334155; ink: root.selected-demo == 2 ? white : #c8d4e0; pressed => { root.select-demo(2); } }
                                Chip { label: "Spaceship"; active: root.selected-demo == 3; fill: #1a2230; active-fill: #78c8ff; stroke: #334155; ink: root.selected-demo == 3 ? #08131c : #c8d4e0; pressed => { root.select-demo(3); } }
                                Chip { label: "Luxury"; active: root.selected-demo == 4; fill: #1a2230; active-fill: #9d6a28; stroke: #334155; ink: root.selected-demo == 4 ? white : #c8d4e0; pressed => { root.select-demo(4); } }
                                Chip { label: "Minimal"; active: root.selected-demo == 5; fill: #1a2230; active-fill: #0d6efd; stroke: #334155; ink: root.selected-demo == 5 ? white : #c8d4e0; pressed => { root.select-demo(5); } }
                                Chip { label: "Inspector"; active: root.selected-demo == 6; fill: #1a2230; active-fill: #f3f4f6; stroke: #334155; ink: root.selected-demo == 6 ? #101828 : #c8d4e0; pressed => { root.select-demo(6); } }
                                Chip { label: "Arcade"; active: root.selected-demo == 7; fill: #1a2230; active-fill: #ff5fa2; stroke: #334155; ink: root.selected-demo == 7 ? #2a0a18 : #c8d4e0; pressed => { root.select-demo(7); } }
                            }

                            Text {
                                text: "One style at a time. Same state, same controls — now in an actual fixed demo viewport.";
                                color: #9fb0c3;
                            }

                            demo-slot := Rectangle {
                                vertical-stretch: 1;
                                min-height: 620px;
                                border-radius: 18px;
                                background: #0f151d;
                                border-width: 1px;
                                border-color: #273241;
                                clip: true;

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 0;
                                    theme-name: "Brutalist console";
                                    subtitle: "Chunky blocks, warning-strip energy, almost industrial.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #f2eadf;
                                    shell-2: #fff7ee;
                                    stroke: #1e1e1e;
                                    accent: #ff5e00;
                                    accent-2: #ffd200;
                                    ink: #111111;
                                    muted: #594b42;
                                    radius: 4;
                                    title-size: 24;
                                    heavy-border: true;
                                    all-caps: true;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 1;
                                    theme-name: "Retro terminal";
                                    subtitle: "Monochrome ops panel with phosphor-screen vibes.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #08140c;
                                    shell-2: #0b1d12;
                                    stroke: #1f8f47;
                                    accent: #49ff8d;
                                    accent-2: #b0ff6d;
                                    ink: #c5ffd8;
                                    muted: #71bf8a;
                                    radius: 8;
                                    title-size: 22;
                                    all-caps: true;
                                    skinny: true;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 2;
                                    theme-name: "Soft cute dashboard";
                                    subtitle: "Rounded toy-like control sheet, friendly and light.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #ffe8f1;
                                    shell-2: #fff5f9;
                                    stroke: #f5a7c6;
                                    accent: #ff6ea8;
                                    accent-2: #8dceff;
                                    ink: #50283c;
                                    muted: #8f5d74;
                                    radius: 28;
                                    title-size: 22;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 3;
                                    theme-name: "Glassy spaceship";
                                    subtitle: "Thin chrome lines, dark gradients, cockpit feel.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #121827;
                                    shell-2: #182131;
                                    stroke: #425574;
                                    accent: #78c8ff;
                                    accent-2: #ae8bff;
                                    ink: #eef6ff;
                                    muted: #a5bdd5;
                                    radius: 20;
                                    title-size: 23;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 4;
                                    theme-name: "Editorial luxury";
                                    subtitle: "Cream surfaces, gold accent, expensive control room.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #f7f1e7;
                                    shell-2: #fcf8f0;
                                    stroke: #cbb48f;
                                    accent: #9d6a28;
                                    accent-2: #3f2d18;
                                    ink: rgb(35, 24, 14);
                                    muted: #76624b;
                                    radius: 18;
                                    title-size: 24;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                ThemeCard {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 5;
                                    theme-name: "Minimal analytical";
                                    subtitle: "White-space heavy, almost pro data-tool or Figma plugin.";
                                    mode-label: root.mode-label;
                                    exposure: root.exposure;
                                    speed: root.speed;
                                    bloom: root.bloom;
                                    shell: #eef2f6;
                                    shell-2: white;
                                    stroke: #cad4de;
                                    accent: #0d6efd;
                                    accent-2: #111827;
                                    ink: #0d1520;
                                    muted: #66778a;
                                    radius: 12;
                                    title-size: 22;
                                    skinny: true;
                                    set-mode(mode) => { root.set-mode(mode); }
                                    bump-exposure(delta) => { root.bump-exposure(delta); }
                                    bump-speed(delta) => { root.bump-speed(delta); }
                                    toggle-bloom() => { root.toggle-bloom(); }
                                    reset-all() => { root.reset-all(); }
                                }

                                Rectangle {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 6;
                                    border-radius: 26px;
                                    background: #e8ebf1;
                                    border-width: 1px;
                                    border-color: #ccd2dc;
                                    Rectangle { x: 8px; y: 10px; width: parent.width; height: parent.height; border-radius: 26px; background: #c8d0dc55; }
                                    Rectangle {
                                        x: 0; y: 0; width: parent.width; height: parent.height; border-radius: 26px; background: #f7f8fb; border-width: 1px; border-color: #d7dde6;
                                        VerticalLayout {
                                            padding: 14px; spacing: 12px;
                                            HorizontalLayout {
                                                spacing: 8px;
                                                Chip { label: "Overview"; active: true; fill: #f4f6fa; active-fill: #101828; stroke: #d5dbe5; ink: white; }
                                                Chip { label: "Viewport"; active: false; fill: #f4f6fa; active-fill: #101828; stroke: #d5dbe5; ink: #364152; }
                                                Chip { label: "Lighting"; active: false; fill: #f4f6fa; active-fill: #101828; stroke: #d5dbe5; ink: #364152; }
                                                Rectangle { horizontal-stretch: 1; }
                                                Text { text: "shadcn-ish panel"; color: #667085; font-size: 13px; }
                                            }
                                            Rectangle {
                                                border-radius: 18px; background: white; border-width: 1px; border-color: #e3e8ef;
                                                VerticalLayout {
                                                    padding: 16px; spacing: 12px;
                                                    Text { text: "Quiet modern inspector"; color: #101828; font-size: 24px; font-weight: 800; }
                                                    Text { text: "More like a real product shell: tabs, softer elevation, restrained outlines, and cleaner spacing."; color: #667085; wrap: word-wrap; }
                                                    HorizontalLayout {
                                                        spacing: 10px;
                                                        Rectangle {
                                                            horizontal-stretch: 1; border-radius: 14px; background: #f8fafc; border-width: 1px; border-color: #e4e7ec;
                                                            VerticalLayout {
                                                                padding: 12px; spacing: 8px;
                                                                Text { text: "Camera mode"; color: #475467; }
                                                                HorizontalLayout {
                                                                    spacing: 6px;
                                                                    Chip { label: "Orbit"; active: root.mode-label == "Orbit"; fill: white; active-fill: #111827; stroke: #d0d5dd; ink: root.mode-label == "Orbit" ? white : #111827; pressed => { root.set-mode(0); } }
                                                                    Chip { label: "Pan"; active: root.mode-label == "Pan"; fill: white; active-fill: #111827; stroke: #d0d5dd; ink: root.mode-label == "Pan" ? white : #111827; pressed => { root.set-mode(1); } }
                                                                    Chip { label: "Inspect"; active: root.mode-label == "Inspect"; fill: white; active-fill: #111827; stroke: #d0d5dd; ink: root.mode-label == "Inspect" ? white : #111827; pressed => { root.set-mode(2); } }
                                                                }
                                                            }
                                                        }
                                                        Rectangle {
                                                            width: 142px; border-radius: 14px; background: #111827; border-width: 1px; border-color: #1f2937;
                                                            VerticalLayout {
                                                                padding: 12px; spacing: 6px;
                                                                Text { text: "Bloom"; color: #98a2b3; }
                                                                Chip { label: root.bloom ? "Enabled" : "Disabled"; active: root.bloom; fill: #1f2937; active-fill: #22c55e; stroke: #344054; ink: root.bloom ? #04130a : white; pressed => { root.toggle-bloom(); } }
                                                            }
                                                        }
                                                    }
                                                    HorizontalLayout {
                                                        spacing: 10px;
                                                        Rectangle { horizontal-stretch: 1; border-radius: 14px; background: #f8fafc; border-width: 1px; border-color: #e4e7ec; HorizontalLayout { padding: 12px; spacing: 8px; Text { text: "Exposure"; color: #475467; } Rectangle { horizontal-stretch: 1; } MicroButton { label: "−"; fill: white; stroke: #d0d5dd; ink: #111827; pressed => { root.bump-exposure(-5); } } Text { text: root.exposure + "%"; color: #111827; font-weight: 700; } MicroButton { label: "+"; fill: #111827; stroke: #111827; ink: white; pressed => { root.bump-exposure(5); } } } }
                                                        Rectangle { horizontal-stretch: 1; border-radius: 14px; background: #f8fafc; border-width: 1px; border-color: #e4e7ec; HorizontalLayout { padding: 12px; spacing: 8px; Text { text: "Speed"; color: #475467; } Rectangle { horizontal-stretch: 1; } MicroButton { label: "−"; fill: white; stroke: #d0d5dd; ink: #111827; pressed => { root.bump-speed(-1); } } Text { text: root.speed + "x"; color: #111827; font-weight: 700; } MicroButton { label: "+"; fill: #111827; stroke: #111827; ink: white; pressed => { root.bump-speed(1); } } } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    x: 0; y: 0; width: parent.width; height: parent.height;
                                    visible: root.selected-demo == 7;
                                    border-radius: 32px;
                                    background: #1b1227;
                                    border-width: 1px;
                                    border-color: #4b2f6b;
                                    clip: true;
                                    Rectangle { x: 16px; y: 14px; width: parent.width - 32px; height: parent.height - 28px; border-radius: 26px; background: #2a173dcc; border-width: 1px; border-color: #7d49b8; }
                                    VerticalLayout {
                                        padding: 16px; spacing: 12px;
                                        HorizontalLayout {
                                            spacing: 8px;
                                            Rectangle { width: 10px; height: 10px; border-radius: 5px; background: #ff5fa2; }
                                            Rectangle { width: 10px; height: 10px; border-radius: 5px; background: #7cf4ff; }
                                            Rectangle { width: 10px; height: 10px; border-radius: 5px; background: #ffe66d; }
                                            Rectangle { horizontal-stretch: 1; }
                                            Text { text: "expressive neon toy"; color: #d6bef8; font-size: 13px; }
                                        }
                                        Rectangle {
                                            border-radius: 26px; background: #241235; border-width: 2px; border-color: #ff5fa2;
                                            VerticalLayout {
                                                padding: 16px; spacing: 12px;
                                                Text { text: "Playful arcade rig"; color: white; font-size: 26px; font-weight: 800; }
                                                Text { text: "Same state, but now it feels like a toy instrument panel. Big pills, saturated accents, and deliberately loud framing."; color: #d9c8f6; wrap: word-wrap; }
                                                HorizontalLayout {
                                                    spacing: 10px;
                                                    Chip { label: "Orbit"; active: root.mode-label == "Orbit"; fill: #35194d; active-fill: #7cf4ff; stroke: #7cf4ff; ink: root.mode-label == "Orbit" ? #07131b : white; pressed => { root.set-mode(0); } }
                                                    Chip { label: "Pan"; active: root.mode-label == "Pan"; fill: #35194d; active-fill: #ffe66d; stroke: #ffe66d; ink: root.mode-label == "Pan" ? #241f06 : white; pressed => { root.set-mode(1); } }
                                                    Chip { label: "Inspect"; active: root.mode-label == "Inspect"; fill: #35194d; active-fill: #ff5fa2; stroke: #ff5fa2; ink: root.mode-label == "Inspect" ? #2a0a18 : white; pressed => { root.set-mode(2); } }
                                                }
                                                HorizontalLayout {
                                                    spacing: 10px;
                                                    Rectangle { horizontal-stretch: 1; border-radius: 18px; background: #311845; border-width: 1px; border-color: #7d49b8; VerticalLayout { padding: 12px; spacing: 8px; Text { text: "Exposure"; color: #f8d0ff; font-weight: 700; } HorizontalLayout { spacing: 8px; MicroButton { label: "−"; fill: #4d2672; stroke: #a56ae9; ink: white; pressed => { root.bump-exposure(-5); } } Rectangle { horizontal-stretch: 1; border-radius: 12px; background: #180c24; border-width: 1px; border-color: #6f3dad; Text { text: root.exposure + "%"; color: white; horizontal-alignment: center; vertical-alignment: center; font-weight: 800; } } MicroButton { label: "+"; fill: #ff5fa2; stroke: #ff5fa2; ink: #300b1d; pressed => { root.bump-exposure(5); } } } } }
                                                    Rectangle { horizontal-stretch: 1; border-radius: 18px; background: #311845; border-width: 1px; border-color: #7d49b8; VerticalLayout { padding: 12px; spacing: 8px; Text { text: "Speed"; color: #c8fbff; font-weight: 700; } HorizontalLayout { spacing: 8px; MicroButton { label: "−"; fill: #4d2672; stroke: #a56ae9; ink: white; pressed => { root.bump-speed(-1); } } Rectangle { horizontal-stretch: 1; border-radius: 12px; background: #180c24; border-width: 1px; border-color: #6f3dad; Text { text: root.speed + "x"; color: white; horizontal-alignment: center; vertical-alignment: center; font-weight: 800; } } MicroButton { label: "+"; fill: #7cf4ff; stroke: #7cf4ff; ink: #04151b; pressed => { root.bump-speed(1); } } } } }
                                                }
                                                HorizontalLayout {
                                                    spacing: 10px;
                                                    Chip { label: root.bloom ? "Bloom ON" : "Bloom OFF"; active: root.bloom; fill: #35194d; active-fill: #ffe66d; stroke: #ffe66d; ink: root.bloom ? #241f06 : white; pressed => { root.toggle-bloom(); } }
                                                    Rectangle { horizontal-stretch: 1; }
                                                    MicroButton { label: "RESET"; min-width: 102px; fill: #ff5fa2; stroke: #ff5fa2; ink: #300b1d; pressed => { root.reset-all(); } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            viewport-box := Rectangle {
                horizontal-stretch: 1;
                vertical-stretch: 1;
                min-width: 420px;
                border-radius: 22px;
                background: #0c1117;
                border-width: 1px;
                border-color: #2a3440;
                clip: true;

                Image {
                    x: 0;
                    y: 0;
                    width: parent.width;
                    height: parent.height;
                    source: root.viewport-image;
                    image-fit: fill;
                }

                Rectangle {
                    x: 18px;
                    y: 18px;
                    width: parent.width - 36px;
                    height: 118px;
                    border-radius: 16px;
                    background: #101823cc;
                    border-width: 1px;
                    border-color: #35506bcc;

                    VerticalLayout {
                        padding: 14px;
                        spacing: 8px;

                        Text {
                            text: "Live viewport / shared control state";
                            color: #eff6ff;
                            font-size: 22px;
                            font-weight: 800;
                        }

                        Text {
                            text: root.status-text;
                            color: #b7c9dc;
                            wrap: word-wrap;
                        }

                        Text {
                            text: "Mode: " + root.mode-label + "    Exposure: " + root.exposure + "%    Speed: " + root.speed + "x    Bloom: " + (root.bloom ? "On" : "Off");
                            color: #7fd0ff;
                            font-weight: 700;
                        }
                    }
                }

                Rectangle {
                    x: 18px;
                    y: parent.height - 118px;
                    width: parent.width - 36px;
                    height: 100px;
                    border-radius: 16px;
                    background: #0f1720cc;
                    border-width: 1px;
                    border-color: #2b3949cc;

                    VerticalLayout {
                        padding: 12px;
                        spacing: 6px;

                        Text {
                            text: "Why this demo is useful";
                            color: #eef4fb;
                            font-weight: 800;
                        }

                        Text {
                            text: "If Slint can carry this many visual personalities around the same interaction model, it is probably stylistically flexible enough for a real app shell around the embedded Rust `wgpu` view.";
                            color: #a7bbcf;
                            wrap: word-wrap;
                        }
                    }
                }
            }
        }
    }
}

struct RendererResources {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct ViewportTexture {
    texture: wgpu::Texture,
    size: (u32, u32),
}

struct ViewportRenderer {
    app: slint::Weak<App>,
    resources: Option<RendererResources>,
    viewport: Option<ViewportTexture>,
    start: Instant,
    enabled: bool,
}

impl ViewportRenderer {
    fn new(app: slint::Weak<App>, enabled: bool) -> Self {
        Self {
            app,
            resources: None,
            viewport: None,
            start: Instant::now(),
            enabled,
        }
    }

    fn handle(&mut self, state: slint::RenderingState, graphics_api: &slint::GraphicsAPI<'_>) {
        if !self.enabled {
            if let Some(app) = self.app.upgrade() {
                app.set_status_text("Accelerated viewport disabled via POC_DISABLE_WGPU=1".into());
            }
            return;
        }

        match (state, graphics_api) {
            (slint::RenderingState::RenderingSetup, slint::GraphicsAPI::WGPU28 { device, .. }) => {
                if self.resources.is_none() {
                    self.resources = Some(Self::create_resources(device));
                }
            }
            (
                slint::RenderingState::BeforeRendering,
                slint::GraphicsAPI::WGPU28 { device, queue, .. },
            ) => {
                self.render(device, queue);
            }
            (slint::RenderingState::AfterRendering, _) => {}
            (slint::RenderingState::RenderingTeardown, _) => {
                self.viewport = None;
                self.resources = None;
            }
            _ => {}
        }
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(app) = self.app.upgrade() else {
            return;
        };

        let width = app.get_viewport_px_width().max(1) as u32;
        let height = app.get_viewport_px_height().max(1) as u32;

        if self.resources.is_none() {
            self.resources = Some(Self::create_resources(device));
        }

        let needs_resize = self
            .viewport
            .as_ref()
            .is_none_or(|viewport| viewport.size != (width, height));

        if needs_resize {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("EmbeddedViewportTexture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: VIEWPORT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let imported_image = slint::Image::try_from(texture.clone())
                .expect("Slint should accept an RGBA8 texture for Image import");

            self.viewport = Some(ViewportTexture {
                texture,
                size: (width, height),
            });

            app.set_viewport_image(imported_image);
            app.set_status_text(slint::format!(
                "{}x{} native wgpu texture inside Slint layout",
                width,
                height
            ));
        }

        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(viewport) = self.viewport.as_ref() else {
            return;
        };

        let uniform_bytes = uniforms_bytes(width, height, self.start.elapsed().as_secs_f32());
        queue.write_buffer(&resources.uniform_buffer, 0, &uniform_bytes);

        let view = viewport
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("EmbeddedViewportEncoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("EmbeddedViewportPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.03,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&resources.pipeline);
            pass.set_bind_group(0, &resources.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }

    fn create_resources(device: &wgpu::Device) -> RendererResources {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("EmbeddedViewportUniformLayout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(16).expect("16 is non-zero")),
                },
                count: None,
            }],
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("EmbeddedViewportUniformBuffer"),
            contents: &uniforms_bytes(1, 1, 0.0),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("EmbeddedViewportBindGroup"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("EmbeddedViewportPipelineLayout"),
            bind_group_layouts: &[&uniform_layout],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("EmbeddedViewportShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("EmbeddedViewportPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        RendererResources {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }
}

fn uniforms_bytes(width: u32, height: u32, time: f32) -> [u8; 16] {
    let values = [width as f32, height as f32, time, 0.0];
    let mut bytes = [0_u8; 16];

    for (index, value) in values.into_iter().enumerate() {
        let start = index * 4;
        bytes[start..start + 4].copy_from_slice(&value.to_ne_bytes());
    }

    bytes
}

fn main() -> Result<(), Box<dyn Error>> {
    let viewport_enabled = !matches!(
        env::var("POC_DISABLE_WGPU").as_deref(),
        Ok("1" | "true" | "yes")
    );

    let backend_selector = slint::BackendSelector::new();
    if viewport_enabled {
        backend_selector
            .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
            .select()?;
    } else {
        backend_selector.select()?;
    }

    let app = App::new()?;
    if viewport_enabled {
        app.set_status_text("Initializing shared wgpu device and viewport texture".into());
    } else {
        app.set_status_text("Accelerated viewport disabled via POC_DISABLE_WGPU=1".into());
    }

    if viewport_enabled {
        let renderer = Rc::new(RefCell::new(ViewportRenderer::new(
            app.as_weak(),
            viewport_enabled,
        )));
        let renderer_for_notifier = Rc::clone(&renderer);

        app.window()
            .set_rendering_notifier(move |state, graphics_api| {
                renderer_for_notifier
                    .borrow_mut()
                    .handle(state, graphics_api);
            })?;

        let animation_timer = slint::Timer::default();
        let app_for_timer = app.as_weak();
        animation_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                if let Some(app) = app_for_timer.upgrade() {
                    app.window().request_redraw();
                }
            },
        );

        app.run()?;
    } else {
        app.run()?;
    }
    Ok(())
}
