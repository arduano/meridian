use std::{
    fs,
    path::{Path, PathBuf},
};

use meridian_core::{
    protocol::CoreCommand,
    render::{
        ProjectorBackgroundConfig, SceneConfig, TextOverlayConfig, TextRowConfig, TextSceneConfig,
        TextStyleConfig,
    },
    spawn_core,
};

const TEST_MIDI_BYTES: &[u8] = &[
    0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x4d, 0x54,
    0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00, 0x90, 0x3c,
    0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30, 0x80, 0x40, 0x40, 0x00, 0xff, 0x2f,
    0x00,
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("meridian-text-scene-final.png"));
    let midi_path = write_test_midi(&output)?;

    let core = spawn_core();
    core.request(CoreCommand::LoadMidi { path: midi_path })?;
    core.request(CoreCommand::SetSceneConfig {
        scene: SceneConfig::Text(example_text_scene()),
    })?;
    core.request(CoreCommand::SetTime { time: 0.25 })?;
    core.request(CoreCommand::SetViewport {
        width: 1280,
        height: 720,
    })?;

    let events = core.request(CoreCommand::SaveFrame {
        output: output.clone(),
        format: None,
        viewport_width: Some(1280),
        viewport_height: Some(720),
        export: Default::default(),
    })?;

    println!("{}", output.display());
    for event in events {
        println!("{event:?}");
    }

    Ok(())
}

fn write_test_midi(output: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let midi_path = output.with_extension("mid");
    fs::write(&midi_path, TEST_MIDI_BYTES)?;
    Ok(midi_path)
}

fn example_text_scene() -> TextSceneConfig {
    TextSceneConfig {
        background: ProjectorBackgroundConfig::None,
        background_color: "#102B1E".into(),
        default_style: "body".into(),
        styles: vec![
            TextStyleConfig {
                name: "title".into(),
                font_family: "Noto Serif".into(),
                font_size: 58,
                color: "#F3FFF9".into(),
                line_spacing: 10.0,
            },
            TextStyleConfig {
                name: "body".into(),
                font_family: "Inter, Noto Sans, sans-serif".into(),
                font_size: 34,
                color: "#D7FBE8".into(),
                line_spacing: 8.0,
            },
            TextStyleConfig {
                name: "mono".into(),
                font_family: "JetBrains Mono, monospace".into(),
                font_size: 28,
                color: "#9FC7FF".into(),
                line_spacing: 6.0,
            },
        ],
        overlays: vec![
            TextOverlayConfig {
                name: "Title".into(),
                x: 0.08,
                y: 0.10,
                width: 0.48,
                style: "body".into(),
                rows: vec![
                    TextRowConfig::PlainText {
                        text: "MERIDIAN".into(),
                        style: Some("title".into()),
                    },
                    TextRowConfig::PlainText {
                        text: "Structured text scene prototype".into(),
                        style: Some("body".into()),
                    },
                ],
                ..Default::default()
            },
            TextOverlayConfig {
                name: "Metrics".into(),
                x: 0.08,
                y: 0.28,
                width: 0.42,
                padding: 18.0,
                row_gap: 10.0,
                background_color: Some("#132031CC".into()),
                style: "body".into(),
                rows: vec![
                    TextRowConfig::PlainText {
                        text: "Time {{time.current|clock}}".into(),
                        style: Some("mono".into()),
                    },
                    TextRowConfig::PlainText {
                        text: "Notes {{notes.passed|int}} / {{notes.total|int}}".into(),
                        style: Some("mono".into()),
                    },
                    TextRowConfig::PlainText {
                        text: "BPM {{tempo.bpm|bpm}}".into(),
                        style: Some("mono".into()),
                    },
                    TextRowConfig::PlainText {
                        text: "Polyphony {{polyphony.current|int}}".into(),
                        style: Some("mono".into()),
                    },
                ],
                ..Default::default()
            },
        ],
    }
}
