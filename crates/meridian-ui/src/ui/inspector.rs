use meridian_core::render::{
    KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig, NoteProjectorConfig,
    PfaTopColor, SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
};

use super::view::InspectorRow;

pub fn rows_for_scene(scene: &SceneConfig) -> Vec<InspectorRow> {
    let mut rows = Vec::new();
    match scene {
        SceneConfig::TwoD(config) => {
            rows.push(row("Scene", "Scene Type", "2D"));
            match &config.keyboard_height {
                KeyboardHeightSpec::ScreenPercent { height } => {
                    rows.push(row(
                        "Layout",
                        "Keyboard Height",
                        format!("{:.1}%", height * 100.0),
                    ));
                    rows.push(row("Layout", "Height Mode", "screen_percent"));
                }
                KeyboardHeightSpec::AspectRatio { ratio } => {
                    rows.push(row("Layout", "Keyboard Height", format!("{ratio:.5}")));
                    rows.push(row("Layout", "Height Mode", "aspect_ratio"));
                }
            }
            rows.extend(note_rows(&config.notes));
            rows.extend(keyboard_rows(&config.keyboard));
        }
        SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(config)) => {
            rows.push(row("Scene", "Scene Type", "3D"));
            rows.push(row("Scene", "Projector", "miditrail"));
            rows.push(row(
                "Camera",
                "FOV",
                format!("{:.2}", config.fov.to_degrees()),
            ));
            rows.push(row(
                "Camera",
                "View Height",
                format!("{:.2}", config.view_height),
            ));
            rows.push(row(
                "Camera",
                "View Offset",
                format!("{:.2}", config.view_offset),
            ));
            rows.push(row(
                "Camera",
                "View Dist",
                format!("{:.2}", config.viewdist),
            ));
            rows.push(row(
                "Scene",
                "Same Width Notes",
                yes_no(config.same_width_notes),
            ));
            rows.push(row("Scene", "Box Notes", yes_no(config.box_notes)));
            rows.push(row("Scene", "Show Keyboard", yes_no(config.show_keyboard)));
            rows.push(row("Scene", "Palette", palette_name(&config.palette)));
        }
    }
    rows
}

fn note_rows(config: &NoteProjectorConfig) -> Vec<InspectorRow> {
    match config {
        NoteProjectorConfig::Flat(config) => vec![
            row("Notes", "Projector", "flat"),
            row("Notes", "Palette", palette_name(&config.palette)),
        ],
        NoteProjectorConfig::Pfa(config) => vec![
            row("Notes", "Projector", "pfa"),
            row("Notes", "Same Width Notes", yes_no(config.same_width_notes)),
            row(
                "Notes",
                "Border Width",
                format!("{:.2}", config.border_width),
            ),
            row("Notes", "Palette", palette_name(&config.palette)),
        ],
    }
}

fn keyboard_rows(config: &KeyboardProjectorConfig) -> Vec<InspectorRow> {
    match config {
        KeyboardProjectorConfig::Flat(_) => vec![row("Keyboard", "Projector", "flat")],
        KeyboardProjectorConfig::Pfa(config) => vec![
            row("Keyboard", "Projector", "pfa"),
            row(
                "Keyboard",
                "Same Width Notes",
                yes_no(config.same_width_notes),
            ),
            row("Keyboard", "Middle C Marker", yes_no(config.middle_c)),
            row("Keyboard", "Top Color", top_color(config.top_color)),
            row(
                "Keyboard",
                "Top Bar RGB",
                format!(
                    "{:.2}, {:.2}, {:.2}",
                    config.top_bar_rgb[0], config.top_bar_rgb[1], config.top_bar_rgb[2]
                ),
            ),
        ],
    }
}

fn row(
    section: impl Into<slint::SharedString>,
    label: impl Into<slint::SharedString>,
    value: impl Into<slint::SharedString>,
) -> InspectorRow {
    InspectorRow {
        section: section.into(),
        label: label.into(),
        value: value.into(),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn top_color(color: PfaTopColor) -> &'static str {
    match color {
        PfaTopColor::Red => "red",
        PfaTopColor::Blue => "blue",
        PfaTopColor::Green => "green",
    }
}

fn palette_name(config: &NotePaletteConfig) -> String {
    match config {
        NotePaletteConfig::DefaultTrackColors => "default_track_colors".into(),
        NotePaletteConfig::ZenithPalette { palette, randomize } => {
            let base = match palette {
                ZenithPaletteSpec::Random => "zenith_random".to_string(),
                ZenithPaletteSpec::RandomGradients => "zenith_random_gradients".to_string(),
                ZenithPaletteSpec::PngFile { path } => path.to_string_lossy().into_owned(),
            };
            format!("{base}{}", if *randomize { " / randomized" } else { "" })
        }
    }
}
