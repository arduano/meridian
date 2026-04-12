use std::sync::{Mutex, OnceLock};

use cosmic_text::{
    Align, Attrs, Buffer, Color, FamilyOwned, FontSystem, Metrics, Shaping, SwashCache, Wrap,
};

use crate::{
    midi::{MIDIFileBase, MIDIFileUnion},
    render::{
        SceneLayout,
        shared::{
            ProjectedScene, SceneLayer, TextAlignment, TextAnchor, TextOverlayConfig, TextRowConfig, TextSceneConfig,
            TextStyleConfig, TextValueFormat, TextValueSource, solid_quad,
        },
    },
};

static TEXT_RASTERIZER: OnceLock<Mutex<TextRasterizer>> = OnceLock::new();

#[derive(Clone, Debug, Default)]
pub struct TextRenderMetrics {
    pub midi_name: Option<String>,
    pub renderer_name: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub current_time_seconds: f64,
    pub midi_length_seconds: f64,
    pub current_tick: f64,
    pub midi_length_tick: f64,
    pub total_notes: u64,
    pub passed_notes: u64,
    pub visible_notes: u64,
    pub active_keys: u64,
    pub current_polyphony: u64,
    pub current_bpm: f64,
    pub current_nps_1s: f64,
    pub current_nps_2s: f64,
}

struct TextRasterizer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

#[derive(Clone)]
struct RowRenderPlan {
    text: String,
    style: TextStyleConfig,
    height_px: f32,
}

impl TextRasterizer {
    fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    fn project(
        &mut self,
        config: &TextSceneConfig,
        layout: &SceneLayout,
        metrics: &TextRenderMetrics,
        scene: &mut ProjectedScene,
    ) {
        let viewport_width = layout.viewport_width.max(1);
        let viewport_height = layout.viewport_height.max(1);
        let viewport_width_f = viewport_width as f32;
        let viewport_height_f = viewport_height as f32;

        for overlay in &config.overlays {
            let inner_width_px = (overlay.resolved_width() * viewport_width_f).max(1.0);
            let plans = overlay
                .rows
                .iter()
                .filter_map(|row| {
                    let text = format_row(row, metrics)?;
                    let style = resolve_style(config, overlay, row);
                    let height_px = self.measure_text_height(&text, &style, inner_width_px, overlay.alignment);
                    Some(RowRenderPlan {
                        text,
                        style,
                        height_px,
                    })
                })
                .collect::<Vec<_>>();
            if plans.is_empty() {
                continue;
            }

            let content_height = plans.iter().map(|plan| plan.height_px).sum::<f32>()
                + overlay.row_gap.max(0.0) * plans.len().saturating_sub(1) as f32;
            let padding = overlay.padding.max(0.0);
            let box_width = inner_width_px + padding * 2.0;
            let box_height = content_height + padding * 2.0;
            let (box_left, box_top) = anchor_top_left(
                overlay.anchor,
                overlay.x,
                overlay.y,
                box_width,
                box_height,
                viewport_width_f,
                viewport_height_f,
            );

            if let Some(background_rgba) = overlay.background_rgba() {
                push_rect_px(
                    scene,
                    viewport_width_f,
                    viewport_height_f,
                    box_left,
                    box_top,
                    box_width,
                    box_height,
                    background_rgba,
                );
            }

            let mut cursor_y = box_top + padding;
            for plan in plans {
                self.draw_text(
                    scene,
                    viewport_width_f,
                    viewport_height_f,
                    box_left + padding,
                    cursor_y,
                    inner_width_px,
                    overlay.alignment,
                    &plan.text,
                    &plan.style,
                );
                cursor_y += plan.height_px + overlay.row_gap.max(0.0);
            }
        }
    }

    fn draw_text(
        &mut self,
        scene: &mut ProjectedScene,
        viewport_width: f32,
        viewport_height: f32,
        x_px: f32,
        y_px: f32,
        width_px: f32,
        alignment: TextAlignment,
        text: &str,
        style: &TextStyleConfig,
    ) {
        let buffer = build_buffer(
            &mut self.font_system,
            text,
            style,
            width_px,
            alignment,
        );
        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            rgba_to_cosmic_color(style.rgba()),
            |px, py, w, h, color| {
                let left = x_px.floor() as i32 + px;
                let top = y_px.floor() as i32 + py;
                let right = left + w as i32;
                let bottom = top + h as i32;
                if right <= 0
                    || bottom <= 0
                    || left as f32 >= viewport_width
                    || top as f32 >= viewport_height
                {
                    return;
                }

                let x1 = left.max(0) as f32 / viewport_width;
                let x2 = right.min(viewport_width as i32) as f32 / viewport_width;
                let y1 = 1.0 - bottom.min(viewport_height as i32) as f32 / viewport_height;
                let y2 = 1.0 - top.max(0) as f32 / viewport_height;
                let rgba = cosmic_color_to_rgba(color);
                if rgba[3] <= 0.0 || x2 <= x1 || y2 <= y1 {
                    return;
                }

                scene.push_quad(SceneLayer::Overlay, solid_quad(x1, y1, x2, y2, rgba));
            },
        );
    }

    fn measure_text_height(
        &mut self,
        text: &str,
        style: &TextStyleConfig,
        width_px: f32,
        alignment: TextAlignment,
    ) -> f32 {
        let buffer = build_buffer(
            &mut self.font_system,
            text,
            style,
            width_px,
            alignment,
        );
        buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0_f32, f32::max)
            .max(style.resolved_font_size() as f32 + style.resolved_line_spacing())
    }
}

pub fn build_text_render_metrics(
    midi: Option<&mut MIDIFileUnion>,
    layout: &SceneLayout,
    current_time: f64,
    visible_notes: usize,
    active_keys: usize,
) -> TextRenderMetrics {
    let mut metrics = TextRenderMetrics {
        renderer_name: renderer_name(&layout.scene).to_string(),
        viewport_width: layout.viewport_width,
        viewport_height: layout.viewport_height,
        current_time_seconds: current_time,
        visible_notes: visible_notes as u64,
        active_keys: active_keys as u64,
        ..TextRenderMetrics::default()
    };
    let Some(midi) = midi else {
        return metrics;
    };

    let _ = midi.get_current_column_views(current_time, layout.view_range, layout.time_space);
    let stats = midi.stats();
    let display_cache = midi.display_cache();
    let tempo_map = display_cache.tempo_map();

    metrics.midi_name = Some(
        midi.signature()
            .filepath
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| midi.signature().filepath.display().to_string()),
    );
    metrics.midi_length_seconds = midi.midi_length().unwrap_or(0.0);
    metrics.total_notes = stats.total_notes.unwrap_or(0);
    metrics.passed_notes = stats.passed_notes.unwrap_or(0);
    metrics.current_tick = tempo_map.tick_at_seconds(current_time);
    metrics.midi_length_tick = tempo_map.tick_at_seconds(metrics.midi_length_seconds);
    metrics.current_polyphony = display_cache.active_notes_at(current_time);
    metrics.current_nps_1s = display_cache.note_starts_between((current_time - 1.0).max(0.0), current_time) as f64;
    metrics.current_nps_2s =
        display_cache.note_starts_between((current_time - 2.0).max(0.0), current_time) as f64
            / 2.0;
    metrics.current_bpm =
        tempo_map.ticks_per_second_at_seconds(current_time) * 60.0 / tempo_map.ppq().max(1) as f64;
    metrics
}

pub(crate) fn project_text_scene(
    config: &TextSceneConfig,
    layout: &SceneLayout,
    metrics: &TextRenderMetrics,
    scene: &mut ProjectedScene,
) {
    scene.push_quad(
        SceneLayer::Background,
        solid_quad(0.0, 0.0, 1.0, 1.0, config.background_rgba()),
    );

    let rasterizer = TEXT_RASTERIZER.get_or_init(|| Mutex::new(TextRasterizer::new()));
    let mut rasterizer = rasterizer.lock().expect("text rasterizer mutex poisoned");
    rasterizer.project(config, layout, metrics, scene);
}

fn renderer_name(scene: &crate::render::SceneConfig) -> &'static str {
    match scene {
        crate::render::SceneConfig::TwoD(_) => "2d",
        crate::render::SceneConfig::ThreeD(_) => "3d",
        crate::render::SceneConfig::Text(_) => "text",
    }
}

fn resolve_style(
    config: &TextSceneConfig,
    overlay: &TextOverlayConfig,
    row: &TextRowConfig,
) -> TextStyleConfig {
    row.style_name()
        .and_then(|name| config.style_named(name).cloned())
        .or_else(|| config.style_named(overlay.resolved_style_name()).cloned())
        .unwrap_or_else(|| config.default_style_config().clone())
}

fn format_row(row: &TextRowConfig, metrics: &TextRenderMetrics) -> Option<String> {
    match row {
        TextRowConfig::PlainText { text, .. } => Some(render_template_text(text, metrics)),
        TextRowConfig::Metric {
            label,
            source,
            format,
            prefix,
            suffix,
            ..
        } => {
            let value = format_value(*source, *format, metrics)?;
            Some(join_nonempty(&[
                (!label.trim().is_empty()).then(|| format!("{}:", label.trim())),
                (!prefix.is_empty()).then(|| prefix.clone()),
                Some(value),
                (!suffix.is_empty()).then(|| suffix.clone()),
            ]))
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
            let primary = format_value(*primary_source, *primary_format, metrics)?;
            let secondary = format_value(
                *secondary_source,
                secondary_format.unwrap_or(*primary_format),
                metrics,
            )?;
            Some(join_nonempty(&[
                (!label.trim().is_empty()).then(|| format!("{}:", label.trim())),
                Some(format!("{primary}{}{secondary}", separator)),
            ]))
        }
    }
}

fn render_template_text(template: &str, metrics: &TextRenderMetrics) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let (literal, after_start) = rest.split_at(start);
        out.push_str(literal);
        let after_start = &after_start[2..];
        let Some(end) = after_start.find("}}") else {
            out.push_str("{{");
            out.push_str(after_start);
            return out;
        };
        let (token_body, after_token) = after_start.split_at(end);
        match render_template_token(token_body.trim(), metrics) {
            Some(value) => out.push_str(&value),
            None => {
                out.push_str("{{");
                out.push_str(token_body);
                out.push_str("}}");
            }
        }
        rest = &after_token[2..];
    }
    out.push_str(rest);
    out
}

fn render_template_token(token_body: &str, metrics: &TextRenderMetrics) -> Option<String> {
    let mut parts = token_body.split('|').map(str::trim);
    let source = parse_template_source(parts.next()?)?;
    let format = parts
        .next()
        .and_then(parse_template_format)
        .unwrap_or_else(|| default_format_for_source(source));
    format_value(source, format, metrics)
}

fn parse_template_source(token: &str) -> Option<TextValueSource> {
    match token {
        "midi.name" | "midi_name" => Some(TextValueSource::MidiName),
        "renderer.name" | "renderer_name" => Some(TextValueSource::RendererName),
        "viewport.width" | "viewport_width" => Some(TextValueSource::ViewportWidth),
        "viewport.height" | "viewport_height" => Some(TextValueSource::ViewportHeight),
        "time.current" | "current_time_seconds" => Some(TextValueSource::CurrentTimeSeconds),
        "time.remaining" | "remaining_time_seconds" => Some(TextValueSource::RemainingTimeSeconds),
        "time.length" | "midi_length_seconds" => Some(TextValueSource::MidiLengthSeconds),
        "tick.current" | "current_tick" => Some(TextValueSource::CurrentTick),
        "tick.remaining" | "remaining_tick" => Some(TextValueSource::RemainingTick),
        "tick.length" | "midi_length_tick" => Some(TextValueSource::MidiLengthTick),
        "notes.total" | "total_notes" => Some(TextValueSource::TotalNotes),
        "notes.passed" | "passed_notes" => Some(TextValueSource::PassedNotes),
        "notes.remaining" | "remaining_notes" => Some(TextValueSource::RemainingNotes),
        "notes.visible" | "visible_notes" => Some(TextValueSource::VisibleNotes),
        "keys.active" | "active_keys" => Some(TextValueSource::ActiveKeys),
        "polyphony.current" | "current_polyphony" => Some(TextValueSource::CurrentPolyphony),
        "tempo.bpm" | "current_bpm" => Some(TextValueSource::CurrentBpm),
        "density.nps1" | "current_nps_1s" => Some(TextValueSource::CurrentNps1s),
        "density.nps2" | "current_nps_2s" => Some(TextValueSource::CurrentNps2s),
        _ => None,
    }
}

fn parse_template_format(token: &str) -> Option<TextValueFormat> {
    match token {
        "raw" => Some(TextValueFormat::Raw),
        "int" | "integer" => Some(TextValueFormat::Integer),
        "decimal1" | "decimal_1" | "0.0" => Some(TextValueFormat::Decimal1),
        "decimal2" | "decimal_2" | "0.00" => Some(TextValueFormat::Decimal2),
        "decimal3" | "decimal_3" | "0.000" => Some(TextValueFormat::Decimal3),
        "clock" | "time" => Some(TextValueFormat::Clock),
        "seconds1" | "seconds_1" | "s1" => Some(TextValueFormat::Seconds1),
        "seconds2" | "seconds_2" | "s2" => Some(TextValueFormat::Seconds2),
        "bpm" => Some(TextValueFormat::Bpm),
        "ticks" => Some(TextValueFormat::Ticks),
        _ => None,
    }
}

fn default_format_for_source(source: TextValueSource) -> TextValueFormat {
    match source {
        TextValueSource::MidiName | TextValueSource::RendererName => TextValueFormat::Raw,
        TextValueSource::ViewportWidth
        | TextValueSource::ViewportHeight
        | TextValueSource::TotalNotes
        | TextValueSource::PassedNotes
        | TextValueSource::RemainingNotes
        | TextValueSource::VisibleNotes
        | TextValueSource::ActiveKeys
        | TextValueSource::CurrentPolyphony => TextValueFormat::Integer,
        TextValueSource::CurrentTimeSeconds
        | TextValueSource::RemainingTimeSeconds
        | TextValueSource::MidiLengthSeconds => TextValueFormat::Clock,
        TextValueSource::CurrentTick
        | TextValueSource::RemainingTick
        | TextValueSource::MidiLengthTick => TextValueFormat::Ticks,
        TextValueSource::CurrentBpm => TextValueFormat::Bpm,
        TextValueSource::CurrentNps1s | TextValueSource::CurrentNps2s => TextValueFormat::Decimal1,
    }
}

fn format_value(
    source: TextValueSource,
    format: TextValueFormat,
    metrics: &TextRenderMetrics,
) -> Option<String> {
    match source {
        TextValueSource::MidiName => metrics.midi_name.clone(),
        TextValueSource::RendererName => Some(metrics.renderer_name.clone()),
        TextValueSource::ViewportWidth => Some(format_numeric(metrics.viewport_width as f64, format)),
        TextValueSource::ViewportHeight => Some(format_numeric(metrics.viewport_height as f64, format)),
        TextValueSource::CurrentTimeSeconds => Some(format_seconds(metrics.current_time_seconds, format)),
        TextValueSource::RemainingTimeSeconds => Some(format_seconds(
            (metrics.midi_length_seconds - metrics.current_time_seconds).max(0.0),
            format,
        )),
        TextValueSource::MidiLengthSeconds => Some(format_seconds(metrics.midi_length_seconds, format)),
        TextValueSource::CurrentTick => Some(format_numeric(metrics.current_tick, format)),
        TextValueSource::RemainingTick => Some(format_numeric(
            (metrics.midi_length_tick - metrics.current_tick).max(0.0),
            format,
        )),
        TextValueSource::MidiLengthTick => Some(format_numeric(metrics.midi_length_tick, format)),
        TextValueSource::TotalNotes => Some(format_numeric(metrics.total_notes as f64, format)),
        TextValueSource::PassedNotes => Some(format_numeric(metrics.passed_notes as f64, format)),
        TextValueSource::RemainingNotes => Some(format_numeric(
            metrics.total_notes.saturating_sub(metrics.passed_notes) as f64,
            format,
        )),
        TextValueSource::VisibleNotes => Some(format_numeric(metrics.visible_notes as f64, format)),
        TextValueSource::ActiveKeys => Some(format_numeric(metrics.active_keys as f64, format)),
        TextValueSource::CurrentPolyphony => {
            Some(format_numeric(metrics.current_polyphony as f64, format))
        }
        TextValueSource::CurrentBpm => Some(format_bpm(metrics.current_bpm, format)),
        TextValueSource::CurrentNps1s => Some(format_numeric(metrics.current_nps_1s, format)),
        TextValueSource::CurrentNps2s => Some(format_numeric(metrics.current_nps_2s, format)),
    }
}

fn format_seconds(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Clock => format_clock(value),
        TextValueFormat::Seconds1 => format!("{value:.1}s"),
        TextValueFormat::Seconds2 => format!("{value:.2}s"),
        TextValueFormat::Bpm => format!("{value:.1} BPM"),
        _ => format_numeric(value, format),
    }
}

fn format_bpm(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Integer => format!("{value:.0} BPM"),
        _ => format!("{value:.1} BPM"),
    }
}

fn format_numeric(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Raw | TextValueFormat::Decimal2 => format!("{value:.2}"),
        TextValueFormat::Integer => format!("{value:.0}"),
        TextValueFormat::Decimal1 => format!("{value:.1}"),
        TextValueFormat::Decimal3 => format!("{value:.3}"),
        TextValueFormat::Clock => format_clock(value),
        TextValueFormat::Seconds1 => format!("{value:.1}s"),
        TextValueFormat::Seconds2 => format!("{value:.2}s"),
        TextValueFormat::Bpm => format!("{value:.1} BPM"),
        TextValueFormat::Ticks => format!("{value:.0} ticks"),
    }
}

fn format_clock(value: f64) -> String {
    let total_ms = (value.max(0.0) * 1000.0).round() as u64;
    let total_seconds = total_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let centiseconds = (total_ms % 1000) / 10;
    format!("{minutes:02}:{seconds:02}.{centiseconds:02}")
}

fn join_nonempty(parts: &[Option<String>]) -> String {
    parts.iter().flatten().cloned().collect::<Vec<_>>().join(" ")
}

fn build_buffer(
    font_system: &mut FontSystem,
    text: &str,
    style: &TextStyleConfig,
    width_px: f32,
    alignment: TextAlignment,
) -> Buffer {
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(
            style.resolved_font_size() as f32,
            style.resolved_font_size() as f32 + style.resolved_line_spacing(),
        ),
    );
    let family = parse_font_family(style.normalized_font_family());
    let attrs = Attrs::new().family(family.as_family());
    buffer.set_wrap(font_system, Wrap::WordOrGlyph);
    buffer.set_size(font_system, Some(width_px.max(1.0)), None);
    buffer.set_text(
        font_system,
        text,
        &attrs,
        Shaping::Advanced,
        Some(match alignment {
            TextAlignment::Left => Align::Left,
            TextAlignment::Center => Align::Center,
            TextAlignment::Right => Align::Right,
        }),
    );
    buffer
}

fn anchor_top_left(
    anchor: TextAnchor,
    x: f32,
    y: f32,
    box_width: f32,
    box_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32) {
    let offset_x = x.clamp(0.0, 1.0) * viewport_width;
    let offset_y = y.clamp(0.0, 1.0) * viewport_height;
    match anchor {
        TextAnchor::TopLeft => (offset_x, offset_y),
        TextAnchor::TopRight => (viewport_width - box_width - offset_x, offset_y),
        TextAnchor::BottomLeft => {
            (offset_x, viewport_height - box_height - offset_y)
        }
        TextAnchor::BottomRight => (
            viewport_width - box_width - offset_x,
            viewport_height - box_height - offset_y,
        ),
        TextAnchor::Center => (
            (viewport_width - box_width) * 0.5 + offset_x - viewport_width * 0.5,
            (viewport_height - box_height) * 0.5 + offset_y - viewport_height * 0.5,
        ),
    }
}

fn push_rect_px(
    scene: &mut ProjectedScene,
    viewport_width: f32,
    viewport_height: f32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
) {
    let x1 = left.max(0.0) / viewport_width;
    let x2 = (left + width).min(viewport_width) / viewport_width;
    let y1 = 1.0 - (top + height).min(viewport_height) / viewport_height;
    let y2 = 1.0 - top.max(0.0) / viewport_height;
    if x2 <= x1 || y2 <= y1 {
        return;
    }
    scene.push_quad(SceneLayer::Overlay, solid_quad(x1, y1, x2, y2, color));
}

fn parse_font_family(value: &str) -> FamilyOwned {
    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "serif" => FamilyOwned::Serif,
        "sans" | "sans-serif" | "sans serif" => FamilyOwned::SansSerif,
        "mono" | "monospace" | "mono-space" => FamilyOwned::Monospace,
        "cursive" => FamilyOwned::Cursive,
        "fantasy" => FamilyOwned::Fantasy,
        _ => FamilyOwned::Name(value.trim().into()),
    }
}

fn rgba_to_cosmic_color(rgba: [f32; 4]) -> Color {
    Color::rgba(
        float_channel_to_u8(rgba[0]),
        float_channel_to_u8(rgba[1]),
        float_channel_to_u8(rgba[2]),
        float_channel_to_u8(rgba[3]),
    )
}

fn cosmic_color_to_rgba(color: Color) -> [f32; 4] {
    let [r, g, b, a] = color.as_rgba();
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

fn float_channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{SceneConfig, SceneLayout};

    #[test]
    fn text_scene_projects_structured_rows() {
        let layout = SceneLayout {
            scene: SceneConfig::Text(TextSceneConfig::default()),
            viewport_width: 320,
            viewport_height: 180,
            ..SceneLayout::default()
        };
        let mut scene = ProjectedScene::default();
        let SceneConfig::Text(config) = &layout.scene else {
            panic!("expected text scene");
        };

        let metrics = TextRenderMetrics {
            renderer_name: "text".into(),
            current_time_seconds: 12.34,
            midi_length_seconds: 98.76,
            total_notes: 1234,
            passed_notes: 234,
            current_tick: 4567.0,
            midi_length_tick: 9999.0,
            current_polyphony: 8,
            current_bpm: 174.2,
            current_nps_1s: 12.0,
            current_nps_2s: 8.5,
            ..TextRenderMetrics::default()
        };
        project_text_scene(config, &layout, &metrics, &mut scene);

        assert_eq!(scene.layer(SceneLayer::Background).len(), 1);
        assert!(scene.layer(SceneLayer::Overlay).len() > 100);
        assert_eq!(scene.visible_notes, 0);
    }

    #[test]
    fn plain_text_rows_expand_template_tokens() {
        let metrics = TextRenderMetrics {
            midi_name: Some("demo.mid".into()),
            renderer_name: "text".into(),
            current_time_seconds: 12.34,
            midi_length_seconds: 98.76,
            total_notes: 1234,
            passed_notes: 234,
            current_bpm: 174.2,
            ..TextRenderMetrics::default()
        };

        let rendered = render_template_text(
            "Now {{time.current|clock}} / {{notes.total|int}} / {{tempo.bpm|bpm}} / {{midi.name}}",
            &metrics,
        );

        assert!(rendered.contains("00:12.34"));
        assert!(rendered.contains("1234"));
        assert!(rendered.contains("174.2 BPM"));
        assert!(rendered.contains("demo.mid"));
    }
}
