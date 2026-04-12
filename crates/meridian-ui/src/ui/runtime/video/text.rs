use super::*;

use meridian_core::render::{
    TextAlignment, TextAnchor, TextRowConfig, TextSceneConfig, TextValueFormat, TextValueSource,
};

pub(super) fn wire_video_text_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    wire_text_overlay_callbacks(app, bridge, shared_state);
    wire_text_row_callbacks(app, bridge, shared_state);
    wire_text_style_callbacks(app, bridge, shared_state);
    wire_text_token_callbacks(app, bridge, shared_state);
}

fn wire_text_overlay_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_text_overlay(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                let SceneConfig::Text(config) = scene else {
                    return;
                };
                ensure_simple_text_scene(config);

                if key.as_str() == "scene_background_color" {
                    config.background_color = value.to_string();
                    return;
                }

                let overlay = &mut config.overlays[0];
                match key.as_str() {
                    "anchor" => overlay.anchor = parse_text_anchor(value.as_str()),
                    "alignment" => overlay.alignment = parse_text_alignment(value.as_str()),
                    "background_color" => {
                        overlay.background_color = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    }
                    "x" => {
                        if let Ok(parsed) = value.parse::<f32>() {
                            overlay.x = parsed;
                        }
                    }
                    "y" => {
                        if let Ok(parsed) = value.parse::<f32>() {
                            overlay.y = parsed;
                        }
                    }
                    "width" => {
                        if let Ok(parsed) = value.parse::<f32>() {
                            overlay.width = parsed;
                        }
                    }
                    _ => {}
                }
            });
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_text_overlay_float(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                let SceneConfig::Text(config) = scene else {
                    return;
                };
                ensure_simple_text_scene(config);

                let overlay = &mut config.overlays[0];
                match key.as_str() {
                    "x" => overlay.x = value,
                    "y" => overlay.y = value,
                    "width" => overlay.width = value,
                    _ => {}
                }
            });
        });
    }
}

fn wire_text_row_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    app.on_update_video_text_row(move |key, value| {
        if key.as_str() != "text" {
            return;
        }
        let Some(app) = app_weak.upgrade() else {
            return;
        };
        update_video_scene(&app, &bridge, &shared_state, move |scene| {
            let SceneConfig::Text(config) = scene else {
                return;
            };
            ensure_simple_text_scene(config);
            let TextRowConfig::PlainText { text, .. } = &mut config.overlays[0].rows[0] else {
                unreachable!("text scene row is normalized before editing");
            };
            *text = value.to_string();
        });
    });
}

fn wire_text_token_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    app.on_insert_video_text_token(move |token| {
        let Some(app) = app_weak.upgrade() else {
            return;
        };
        update_video_scene(&app, &bridge, &shared_state, move |scene| {
            let SceneConfig::Text(config) = scene else {
                return;
            };
            ensure_simple_text_scene(config);
            let TextRowConfig::PlainText { text, .. } = &mut config.overlays[0].rows[0] else {
                unreachable!("text scene row is normalized before editing");
            };
            if !text.is_empty()
                && !text.ends_with(' ')
                && !text.ends_with('\n')
                && !token.starts_with(' ')
                && !token.starts_with('\n')
                && !token.starts_with(',')
                && !token.starts_with('.')
            {
                text.push(' ');
            }
            text.push_str(token.as_str());
        });
    });
}

fn wire_text_style_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_text_style(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                let SceneConfig::Text(config) = scene else {
                    return;
                };
                ensure_simple_text_scene(config);
                let style = &mut config.styles[0];
                match key.as_str() {
                    "font_family" => style.font_family = value.to_string(),
                    "color" => style.color = value.to_string(),
                    "font_size" => {
                        if let Ok(parsed) = value.parse::<f32>() {
                            style.font_size = parsed.max(1.0).round() as u32;
                        }
                    }
                    "line_spacing" => {
                        if let Ok(parsed) = value.parse::<f32>() {
                            style.line_spacing = parsed;
                        }
                    }
                    _ => {}
                }
            });
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_text_style_float(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                let SceneConfig::Text(config) = scene else {
                    return;
                };
                ensure_simple_text_scene(config);
                let style = &mut config.styles[0];
                match key.as_str() {
                    "font_size" => style.font_size = value.max(1.0).round() as u32,
                    "line_spacing" => style.line_spacing = value,
                    _ => {}
                }
            });
        });
    }
}

fn parse_text_anchor(value: &str) -> TextAnchor {
    match value {
        "top_right" => TextAnchor::TopRight,
        "bottom_left" => TextAnchor::BottomLeft,
        "bottom_right" => TextAnchor::BottomRight,
        "center" => TextAnchor::Center,
        _ => TextAnchor::TopLeft,
    }
}

fn parse_text_alignment(value: &str) -> TextAlignment {
    match value {
        "center" => TextAlignment::Center,
        "right" => TextAlignment::Right,
        _ => TextAlignment::Left,
    }
}

fn ensure_simple_text_scene(config: &mut TextSceneConfig) {
    let template = config
        .overlays
        .iter()
        .map(|overlay| {
            overlay
                .rows
                .iter()
                .map(row_to_template)
                .filter(|row| !row.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|overlay| !overlay.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut overlay = config.overlays.first().cloned().unwrap_or_default();
    let mut style = config
        .style_named(overlay.resolved_style_name())
        .or_else(|| config.style_named(config.default_style.as_str()))
        .or_else(|| config.style_named("body"))
        .or_else(|| config.styles.first())
        .cloned()
        .unwrap_or_default();

    style.name = "body".to_string();
    overlay.style = "body".to_string();
    overlay.rows = vec![TextRowConfig::PlainText {
        text: template,
        style: None,
    }];

    config.default_style = "body".to_string();
    config.styles = vec![style];
    config.overlays = vec![overlay];
}

fn row_to_template(row: &TextRowConfig) -> String {
    match row {
        TextRowConfig::PlainText { text, .. } => text.clone(),
        TextRowConfig::Metric {
            label,
            source,
            format,
            prefix,
            suffix,
            ..
        } => {
            let mut out = String::new();
            if !label.trim().is_empty() {
                out.push_str(label.trim());
                out.push_str(": ");
            }
            if !prefix.is_empty() {
                out.push_str(prefix);
            }
            out.push_str(&template_token(*source, Some(*format)));
            if !suffix.is_empty() {
                out.push_str(suffix);
            }
            out
        }
        TextRowConfig::MetricPair {
            label,
            primary_source,
            primary_format,
            secondary_source,
            secondary_format,
            separator,
            ..
        } => {
            let mut out = String::new();
            if !label.trim().is_empty() {
                out.push_str(label.trim());
                out.push_str(": ");
            }
            out.push_str(&template_token(*primary_source, Some(*primary_format)));
            out.push_str(separator);
            out.push_str(&template_token(
                *secondary_source,
                secondary_format.or(Some(*primary_format)),
            ));
            out
        }
    }
}

fn template_token(source: TextValueSource, format: Option<TextValueFormat>) -> String {
    let source = match source {
        TextValueSource::MidiName => "midi.name",
        TextValueSource::RendererName => "renderer.name",
        TextValueSource::ViewportWidth => "viewport.width",
        TextValueSource::ViewportHeight => "viewport.height",
        TextValueSource::CurrentTimeSeconds => "time.current",
        TextValueSource::RemainingTimeSeconds => "time.remaining",
        TextValueSource::MidiLengthSeconds => "time.length",
        TextValueSource::CurrentTick => "tick.current",
        TextValueSource::RemainingTick => "tick.remaining",
        TextValueSource::MidiLengthTick => "tick.length",
        TextValueSource::TotalNotes => "notes.total",
        TextValueSource::PassedNotes => "notes.passed",
        TextValueSource::RemainingNotes => "notes.remaining",
        TextValueSource::VisibleNotes => "notes.visible",
        TextValueSource::ActiveKeys => "keys.active",
        TextValueSource::CurrentPolyphony => "polyphony.current",
        TextValueSource::CurrentBpm => "tempo.bpm",
        TextValueSource::CurrentNps1s => "density.nps1",
        TextValueSource::CurrentNps2s => "density.nps2",
    };
    match format {
        Some(format) => format!("{{{{{source}|{}}}}}", template_format_name(format)),
        None => format!("{{{{{source}}}}}"),
    }
}

fn template_format_name(format: TextValueFormat) -> &'static str {
    match format {
        TextValueFormat::Raw => "raw",
        TextValueFormat::Integer => "int",
        TextValueFormat::Decimal1 => "0.0",
        TextValueFormat::Decimal2 => "0.00",
        TextValueFormat::Decimal3 => "0.000",
        TextValueFormat::Clock => "clock",
        TextValueFormat::Seconds1 => "s1",
        TextValueFormat::Seconds2 => "s2",
        TextValueFormat::Bpm => "bpm",
        TextValueFormat::Ticks => "ticks",
    }
}
