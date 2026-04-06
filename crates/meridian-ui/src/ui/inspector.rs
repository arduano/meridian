use meridian_core::render::{
    KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig, NoteProjectorConfig,
    ProjectorBackgroundConfig, ProjectorBackgroundScalingMode, RendererKind, SceneConfig,
    ThreeDSceneConfig, ZenithPaletteSpec,
};

use super::{
    editor_meta::{UiControlKind, projector_editor_meta},
    view::InspectorRow,
};

pub fn rows_for_scene(scene: &SceneConfig) -> Vec<InspectorRow> {
    let mut rows = Vec::new();
    match scene {
        SceneConfig::TwoD(config) => {
            rows.push(row("Scene", "Scene Type", "2D"));
            rows.extend(background_rows(&config.background));
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
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            rows.push(row("Scene", "Scene Type", "3D"));
            rows.push(row("Scene", "Projector", "piano_trail_classic"));
            rows.extend(background_rows(&config.background));
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
            rows.extend(metadata_rows(RendererKind::PianoTrailClassic));
        }
    }
    rows
}

fn note_rows(config: &NoteProjectorConfig) -> Vec<InspectorRow> {
    match config {
        NoteProjectorConfig::Flat(config) => {
            let mut rows = vec![
                row("Notes", "Projector", "flat"),
                row("Notes", "Palette", palette_name(&config.palette)),
            ];
            rows.extend(metadata_rows(RendererKind::Flat));
            rows
        }
        NoteProjectorConfig::Pfa(config) => {
            let mut rows = vec![
                row("Notes", "Projector", "pfa"),
                row("Notes", "Same Width Notes", yes_no(config.same_width_notes)),
                row(
                    "Notes",
                    "Border Width",
                    format!("{:.2}", config.border_width),
                ),
                row("Notes", "Palette", palette_name(&config.palette)),
            ];
            rows.extend(metadata_rows(RendererKind::Pfa));
            rows
        }
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
            row("Keyboard", "Top Bar Color", config.top_bar_color.clone()),
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

fn background_rows(config: &ProjectorBackgroundConfig) -> Vec<InspectorRow> {
    match config {
        ProjectorBackgroundConfig::None => vec![row("Background", "Source", "none")],
        ProjectorBackgroundConfig::PngFile { path, scaling } => vec![
            row("Background", "Source", "png_file"),
            row(
                "Background",
                "Scaling",
                match scaling {
                    ProjectorBackgroundScalingMode::Stretch => "stretch",
                    ProjectorBackgroundScalingMode::Cover => "cover",
                },
            ),
            row("Background", "Path", path.clone()),
        ],
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
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

fn metadata_rows(renderer: RendererKind) -> Vec<InspectorRow> {
    let meta = projector_editor_meta(renderer);
    let mut rows = vec![row("Editor", "Projector Family", meta.label)];
    rows.extend(meta.fields.iter().map(|field| {
        row(
            field.section,
            format!("{} Control ({})", field.label, field.key),
            format!(
                "{} / default {}",
                control_kind_name(field.control),
                field.default_hint
            ),
        )
    }));
    rows
}

fn control_kind_name(kind: UiControlKind) -> String {
    match kind {
        UiControlKind::Toggle => "toggle".into(),
        UiControlKind::Slider { min, max } => format!("slider [{min:.1}, {max:.1}]"),
        UiControlKind::Choice => "choice".into(),
        UiControlKind::Text => "text".into(),
    }
}
