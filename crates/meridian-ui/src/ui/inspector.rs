use meridian_core::render::{
    KeyboardHeightSpec, KeyboardProjectorConfig, NoteProjectorConfig, PfaTopColor, SceneConfig,
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
        SceneConfig::ThreeD(_) => {
            rows.push(row("Scene", "Scene Type", "3D"));
            rows.push(row("Scene", "Status", "Reserved for future implementation"));
        }
    }
    rows
}

fn note_rows(config: &NoteProjectorConfig) -> Vec<InspectorRow> {
    match config {
        NoteProjectorConfig::Flat(_) => vec![row("Notes", "Projector", "flat")],
        NoteProjectorConfig::Pfa(config) => vec![
            row("Notes", "Projector", "pfa"),
            row("Notes", "Same Width Notes", yes_no(config.same_width_notes)),
            row(
                "Notes",
                "Border Width",
                format!("{:.2}", config.border_width),
            ),
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
