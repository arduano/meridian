use std::sync::{Mutex, OnceLock};

use cosmic_text::{
    Align, Attrs, Buffer, Color, FamilyOwned, FontSystem, Metrics, Shaping, SwashCache, Wrap,
};

use crate::{
    midi::{
        MIDIFileBase, MIDIFileUnion, MIDINoteColumnView, MIDINoteViews, views::MIDIFileViewsUnion,
    },
    render::{
        SceneLayout,
        shared::{
            ProjectedScene, SceneLayer, TextAlignment, TextAnchor, TextOverlayConfig,
            TextRowConfig, TextSceneConfig, TextStyleConfig, TextValueFormat, TextValueSource,
            for_each_visible_note, normalized_key_range, solid_quad,
        },
    },
};

static TEXT_RASTERIZER: OnceLock<Mutex<TextRasterizer>> = OnceLock::new();
const TEXT_REFERENCE_WIDTH: f32 = 1920.0;
const TEXT_REFERENCE_HEIGHT: f32 = 1080.0;

#[derive(Clone, Debug, Default)]
pub struct TextRenderMetrics {
    pub midi_name: Option<String>,
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

#[derive(Clone, Copy, Debug)]
enum TemplateFormatSpec {
    Preset(TextValueFormat),
    Number(NumberFormatOptions),
}

#[derive(Clone, Copy, Debug)]
struct NumberFormatOptions {
    min_decimals: usize,
    max_decimals: usize,
    grouping: bool,
    locale: NumberLocale,
}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            min_decimals: 0,
            max_decimals: 3,
            grouping: true,
            locale: NumberLocale::En,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum NumberLocale {
    #[default]
    En,
    De,
    Fr,
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
        let text_scale = text_reference_scale(viewport_width_f, viewport_height_f);

        for overlay in &config.overlays {
            let inner_width_px = (overlay.resolved_width() * viewport_width_f).max(1.0);
            let row_gap_px = overlay.row_gap.max(0.0) * text_scale;
            let plans = overlay
                .rows
                .iter()
                .filter_map(|row| {
                    let text = format_row(row, metrics)?;
                    let style = resolve_style(config, overlay, row);
                    let height_px = self.measure_text_height(
                        &text,
                        &style,
                        inner_width_px,
                        overlay.alignment,
                        text_scale,
                    );
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
                + row_gap_px * plans.len().saturating_sub(1) as f32;
            let padding = overlay.padding.max(0.0) * text_scale;
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
                    text_scale,
                );
                cursor_y += plan.height_px + row_gap_px;
            }
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Text drawing keeps the projected box geometry and style inputs explicit at the call site."
    )]
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
        text_scale: f32,
    ) {
        let buffer = build_buffer(
            &mut self.font_system,
            text,
            style,
            width_px,
            alignment,
            text_scale,
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
        text_scale: f32,
    ) -> f32 {
        let buffer = build_buffer(
            &mut self.font_system,
            text,
            style,
            width_px,
            alignment,
            text_scale,
        );
        buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0_f32, f32::max)
            .max(scaled_font_size(style, text_scale) + scaled_line_spacing(style, text_scale))
    }
}

pub fn build_text_render_metrics(
    midi: Option<&mut MIDIFileUnion>,
    layout: &SceneLayout,
    current_time: f64,
) -> TextRenderMetrics {
    let mut metrics = TextRenderMetrics {
        viewport_width: layout.viewport_width,
        viewport_height: layout.viewport_height,
        current_time_seconds: current_time,
        ..TextRenderMetrics::default()
    };
    let Some(midi) = midi else {
        return metrics;
    };

    let stats = midi.stats();
    let display_cache = midi.display_cache();
    let tempo_map = display_cache.tempo_map();
    let current_tick = tempo_map.tick_at_seconds(current_time);
    let midi_length_seconds = midi.midi_length().unwrap_or(0.0);
    let midi_length_tick = tempo_map.tick_at_seconds(midi_length_seconds);
    let current_polyphony = display_cache.active_notes_at(current_time);
    let current_nps_1s =
        display_cache.note_starts_between((current_time - 1.0).max(0.0), current_time) as f64;
    let current_nps_2s =
        display_cache.note_starts_between((current_time - 2.0).max(0.0), current_time) as f64 / 2.0;
    let current_bpm =
        tempo_map.ticks_per_second_at_seconds(current_time) * 60.0 / tempo_map.ppq().max(1) as f64;
    let (visible_notes, active_keys) = {
        let views =
            midi.get_current_column_views(current_time, layout.view_range, layout.time_space);
        measure_current_view_metrics(&views, layout)
    };

    metrics.midi_name = Some(
        midi.signature()
            .filepath
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| midi.signature().filepath.display().to_string()),
    );
    metrics.midi_length_seconds = midi_length_seconds;
    metrics.total_notes = stats.total_notes.unwrap_or(0);
    metrics.passed_notes = stats.passed_notes.unwrap_or(0);
    metrics.visible_notes = visible_notes;
    metrics.active_keys = active_keys;
    metrics.current_tick = current_tick;
    metrics.midi_length_tick = midi_length_tick;
    metrics.current_polyphony = current_polyphony;
    metrics.current_nps_1s = current_nps_1s;
    metrics.current_nps_2s = current_nps_2s;
    metrics.current_bpm = current_bpm;
    metrics
}

fn measure_current_view_metrics(
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
) -> (u64, u64) {
    let render_end = views.range().length() as f32;
    let (first_key, last_key_exclusive) = normalized_key_range(layout.first_key, layout.last_key);
    let mut visible_notes = 0_u64;
    let mut active_keys = 0_u64;

    match views {
        MIDIFileViewsUnion::InRam(in_ram_views) => {
            for key in first_key..last_key_exclusive {
                let column = in_ram_views.get_column(key);
                let mut key_active = false;
                visible_notes += for_each_visible_note(
                    column.iterate_displaced_notes(),
                    0.0,
                    render_end,
                    false,
                    false,
                    |note| {
                        key_active |= note.active;
                    },
                ) as u64;
                if key_active {
                    active_keys += 1;
                }
            }
        }
    }

    (visible_notes, active_keys)
}

pub(crate) fn project_text_scene(
    config: &TextSceneConfig,
    layout: &SceneLayout,
    metrics: &TextRenderMetrics,
    scene: &mut ProjectedScene,
) {
    let background = config.background_rgba();
    if background[3] > 0.0 {
        scene.push_quad(
            SceneLayer::Background,
            solid_quad(0.0, 0.0, 1.0, 1.0, background),
        );
    }

    let rasterizer = TEXT_RASTERIZER.get_or_init(|| Mutex::new(TextRasterizer::new()));
    let mut rasterizer = rasterizer.lock().expect("text rasterizer mutex poisoned");
    rasterizer.project(config, layout, metrics, scene);
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
    let mut parts = token_body.splitn(2, '|').map(str::trim);
    let source = parse_template_source(parts.next()?)?;
    let format = match parts.next() {
        Some(token) => parse_template_format_spec(token)?,
        None => TemplateFormatSpec::Preset(default_format_for_source(source)),
    };
    format_value_with_spec(source, format, metrics)
}

fn parse_template_source(token: &str) -> Option<TextValueSource> {
    match token {
        "midi.name" => Some(TextValueSource::MidiName),
        "viewport.width" => Some(TextValueSource::ViewportWidth),
        "viewport.height" => Some(TextValueSource::ViewportHeight),
        "time.current" => Some(TextValueSource::CurrentTimeSeconds),
        "time.remaining" => Some(TextValueSource::RemainingTimeSeconds),
        "time.length" => Some(TextValueSource::MidiLengthSeconds),
        "tick.current" => Some(TextValueSource::CurrentTick),
        "tick.remaining" => Some(TextValueSource::RemainingTick),
        "tick.length" => Some(TextValueSource::MidiLengthTick),
        "notes.total" => Some(TextValueSource::TotalNotes),
        "notes.passed" => Some(TextValueSource::PassedNotes),
        "notes.remaining" => Some(TextValueSource::RemainingNotes),
        "notes.visible" => Some(TextValueSource::VisibleNotes),
        "keys.active" => Some(TextValueSource::ActiveKeys),
        "polyphony.current" => Some(TextValueSource::CurrentPolyphony),
        "tempo.bpm" => Some(TextValueSource::CurrentBpm),
        "density.nps.1s" => Some(TextValueSource::CurrentNps1s),
        "density.nps.2s" => Some(TextValueSource::CurrentNps2s),
        _ => None,
    }
}

fn parse_template_format_spec(token: &str) -> Option<TemplateFormatSpec> {
    if let Some(number) = parse_number_format(token) {
        return Some(TemplateFormatSpec::Number(number));
    }
    parse_template_format(token).map(TemplateFormatSpec::Preset)
}

fn parse_template_format(token: &str) -> Option<TextValueFormat> {
    match token {
        "clock" => Some(TextValueFormat::Clock),
        "clock_millis" => Some(TextValueFormat::ClockMillis),
        _ => None,
    }
}

fn parse_number_format(token: &str) -> Option<NumberFormatOptions> {
    let token = token.trim();
    if token == "number" {
        return Some(NumberFormatOptions::default());
    }
    let args = token.strip_prefix("number(")?.strip_suffix(')')?.trim();
    let mut options = NumberFormatOptions::default();
    if args.is_empty() {
        return Some(options);
    }

    for part in args.split(',') {
        let (key, value) = part.split_once(':')?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "decimals" => {
                let decimals = parse_decimal_count(value)?;
                options.min_decimals = decimals;
                options.max_decimals = decimals;
            }
            "min_decimals" => options.min_decimals = parse_decimal_count(value)?,
            "max_decimals" => options.max_decimals = parse_decimal_count(value)?,
            "group" | "grouping" => options.grouping = parse_bool_flag(value)?,
            "locale" => options.locale = parse_number_locale(value)?,
            _ => return None,
        }
    }

    if options.min_decimals > options.max_decimals {
        options.max_decimals = options.min_decimals;
    }

    Some(options)
}

fn parse_decimal_count(value: &str) -> Option<usize> {
    value.parse::<usize>().ok().map(|value| value.min(12))
}

fn parse_bool_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" => Some(true),
        "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

fn parse_number_locale(value: &str) -> Option<NumberLocale> {
    let normalized = value.trim().replace('_', "-").to_ascii_lowercase();
    match normalized.as_str() {
        "en" | "en-us" | "en-gb" => Some(NumberLocale::En),
        "de" | "de-de" => Some(NumberLocale::De),
        "fr" | "fr-fr" => Some(NumberLocale::Fr),
        _ => None,
    }
}

fn default_format_for_source(source: TextValueSource) -> TextValueFormat {
    match source {
        TextValueSource::MidiName => TextValueFormat::Raw,
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
        TextValueSource::ViewportWidth => {
            Some(format_numeric(metrics.viewport_width as f64, format))
        }
        TextValueSource::ViewportHeight => {
            Some(format_numeric(metrics.viewport_height as f64, format))
        }
        TextValueSource::CurrentTimeSeconds => {
            Some(format_seconds(metrics.current_time_seconds, format))
        }
        TextValueSource::RemainingTimeSeconds => Some(format_seconds(
            (metrics.midi_length_seconds - metrics.current_time_seconds).max(0.0),
            format,
        )),
        TextValueSource::MidiLengthSeconds => {
            Some(format_seconds(metrics.midi_length_seconds, format))
        }
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

fn format_value_with_spec(
    source: TextValueSource,
    format: TemplateFormatSpec,
    metrics: &TextRenderMetrics,
) -> Option<String> {
    match format {
        TemplateFormatSpec::Preset(format) => format_value(source, format, metrics),
        TemplateFormatSpec::Number(options) => {
            numeric_value(source, metrics).map(|value| format_number_options(value, options))
        }
    }
}

fn numeric_value(source: TextValueSource, metrics: &TextRenderMetrics) -> Option<f64> {
    match source {
        TextValueSource::MidiName => None,
        TextValueSource::ViewportWidth => Some(metrics.viewport_width as f64),
        TextValueSource::ViewportHeight => Some(metrics.viewport_height as f64),
        TextValueSource::CurrentTimeSeconds => Some(metrics.current_time_seconds),
        TextValueSource::RemainingTimeSeconds => {
            Some((metrics.midi_length_seconds - metrics.current_time_seconds).max(0.0))
        }
        TextValueSource::MidiLengthSeconds => Some(metrics.midi_length_seconds),
        TextValueSource::CurrentTick => Some(metrics.current_tick),
        TextValueSource::RemainingTick => {
            Some((metrics.midi_length_tick - metrics.current_tick).max(0.0))
        }
        TextValueSource::MidiLengthTick => Some(metrics.midi_length_tick),
        TextValueSource::TotalNotes => Some(metrics.total_notes as f64),
        TextValueSource::PassedNotes => Some(metrics.passed_notes as f64),
        TextValueSource::RemainingNotes => {
            Some(metrics.total_notes.saturating_sub(metrics.passed_notes) as f64)
        }
        TextValueSource::VisibleNotes => Some(metrics.visible_notes as f64),
        TextValueSource::ActiveKeys => Some(metrics.active_keys as f64),
        TextValueSource::CurrentPolyphony => Some(metrics.current_polyphony as f64),
        TextValueSource::CurrentBpm => Some(metrics.current_bpm),
        TextValueSource::CurrentNps1s => Some(metrics.current_nps_1s),
        TextValueSource::CurrentNps2s => Some(metrics.current_nps_2s),
    }
}

fn format_seconds(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Clock => format_clock(value),
        TextValueFormat::ClockMillis => format_clock_millis(value),
        TextValueFormat::Seconds1 => format_fixed_numeric(value, 1),
        TextValueFormat::Seconds2 => format_fixed_numeric(value, 2),
        TextValueFormat::Bpm => format_fixed_numeric(value, 1),
        _ => format_numeric(value, format),
    }
}

fn format_bpm(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Raw => format_compact_numeric(value),
        TextValueFormat::Integer | TextValueFormat::Ticks => format_grouped_integer(value),
        TextValueFormat::Decimal2 | TextValueFormat::Seconds2 => format_fixed_numeric(value, 2),
        TextValueFormat::Decimal3 => format_fixed_numeric(value, 3),
        TextValueFormat::Clock => format_clock(value),
        TextValueFormat::ClockMillis => format_clock_millis(value),
        _ => format_fixed_numeric(value, 1),
    }
}

fn format_numeric(value: f64, format: TextValueFormat) -> String {
    match format {
        TextValueFormat::Raw => format_compact_numeric(value),
        TextValueFormat::Integer => format_grouped_integer(value),
        TextValueFormat::Decimal1 => format_fixed_numeric(value, 1),
        TextValueFormat::Decimal2 => format_fixed_numeric(value, 2),
        TextValueFormat::Decimal3 => format_fixed_numeric(value, 3),
        TextValueFormat::Clock => format_clock(value),
        TextValueFormat::ClockMillis => format_clock_millis(value),
        TextValueFormat::Seconds1 => format_fixed_numeric(value, 1),
        TextValueFormat::Seconds2 => format_fixed_numeric(value, 2),
        TextValueFormat::Bpm => format_fixed_numeric(value, 1),
        TextValueFormat::Ticks => format_grouped_integer(value),
    }
}

fn format_clock(value: f64) -> String {
    let total_seconds = value.max(0.0).floor() as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

fn format_clock_millis(value: f64) -> String {
    let total_ms = (value.max(0.0) * 1000.0).round() as u64;
    let total_seconds = total_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = total_ms % 1000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

fn format_compact_numeric(value: f64) -> String {
    format_number_options(value, NumberFormatOptions::default())
}

fn format_fixed_numeric(value: f64, decimals: usize) -> String {
    format_number_options(
        value,
        NumberFormatOptions {
            min_decimals: decimals,
            max_decimals: decimals,
            ..NumberFormatOptions::default()
        },
    )
}

fn format_grouped_integer(value: f64) -> String {
    format_number_options(
        value,
        NumberFormatOptions {
            min_decimals: 0,
            max_decimals: 0,
            ..NumberFormatOptions::default()
        },
    )
}

fn format_number_options(value: f64, options: NumberFormatOptions) -> String {
    let rounded = if options.max_decimals == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.precision$}", precision = options.max_decimals)
    };
    let (sign, rest) = if let Some(stripped) = rounded.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", rounded.as_str())
    };
    let (whole, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    let mut fraction = fraction.to_string();
    while fraction.len() > options.min_decimals && fraction.ends_with('0') {
        fraction.pop();
    }
    format_grouped_number_parts(sign, whole, &fraction, options)
}

fn format_grouped_number_parts(
    sign: &str,
    whole: &str,
    fraction: &str,
    options: NumberFormatOptions,
) -> String {
    let (group_sep, decimal_sep) = match options.locale {
        NumberLocale::En => (",", "."),
        NumberLocale::De => (".", ","),
        NumberLocale::Fr => (" ", ","),
    };
    let whole = if options.grouping {
        let mut grouped_reversed = String::with_capacity(whole.len() + whole.len() / 3);
        for (index, ch) in whole.chars().rev().enumerate() {
            if index != 0 && index % 3 == 0 {
                grouped_reversed.push_str(group_sep);
            }
            grouped_reversed.push(ch);
        }
        grouped_reversed.chars().rev().collect::<String>()
    } else {
        whole.to_string()
    };
    if fraction.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}{decimal_sep}{fraction}")
    }
}

fn join_nonempty(parts: &[Option<String>]) -> String {
    parts
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_buffer(
    font_system: &mut FontSystem,
    text: &str,
    style: &TextStyleConfig,
    width_px: f32,
    alignment: TextAlignment,
    text_scale: f32,
) -> Buffer {
    let font_size = scaled_font_size(style, text_scale);
    let line_height = font_size + scaled_line_spacing(style, text_scale);
    let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
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

fn text_reference_scale(viewport_width: f32, viewport_height: f32) -> f32 {
    let width_scale = viewport_width.max(1.0) / TEXT_REFERENCE_WIDTH;
    let height_scale = viewport_height.max(1.0) / TEXT_REFERENCE_HEIGHT;
    width_scale.min(height_scale).max(f32::EPSILON)
}

fn scaled_font_size(style: &TextStyleConfig, text_scale: f32) -> f32 {
    (style.resolved_font_size() as f32 * text_scale).max(1.0)
}

fn scaled_line_spacing(style: &TextStyleConfig, text_scale: f32) -> f32 {
    style.resolved_line_spacing() * text_scale
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
        TextAnchor::BottomLeft => (offset_x, viewport_height - box_height - offset_y),
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

#[expect(
    clippy::too_many_arguments,
    reason = "Overlay rectangle helpers operate on already-expanded pixel geometry."
)]
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
            current_time_seconds: 12.34,
            midi_length_seconds: 98.76,
            total_notes: 1234,
            passed_notes: 234,
            current_tick: 4567.0,
            current_bpm: 174.2,
            current_nps_1s: 12.5,
            ..TextRenderMetrics::default()
        };

        let rendered = render_template_text(
            "Now {{time.current|clock}} / {{notes.total|number(decimals:0)}} / {{tempo.bpm|number(decimals:1)}} / {{midi.name}} / {{tick.current|number(decimals:0)}} / {{time.current|clock_millis}} / {{density.nps.1s|number(decimals:1)}}",
            &metrics,
        );

        assert!(rendered.contains("00:12"));
        assert!(rendered.contains("1,234"));
        assert!(rendered.contains("174.2"));
        assert!(rendered.contains("demo.mid"));
        assert!(rendered.contains("4,567"));
        assert!(rendered.contains("00:12.340"));
        assert!(rendered.contains("12.5"));
    }

    #[test]
    fn number_format_supports_grouping_and_locale_modifiers() {
        let metrics = TextRenderMetrics {
            total_notes: 12345,
            current_bpm: 174.25,
            ..TextRenderMetrics::default()
        };

        let rendered = render_template_text(
            "{{notes.total|number(decimals:0, group:false)}} / {{notes.total|number(decimals:0, locale:de-DE)}} / {{notes.total|number(decimals:0, locale:fr_FR)}} / {{tempo.bpm|number(min_decimals:1, max_decimals:3, locale:DE-de, group:OFF)}}",
            &metrics,
        );

        assert!(rendered.contains("12345"));
        assert!(rendered.contains("12.345"));
        assert!(rendered.contains("12 345"));
        assert!(rendered.contains("174,25"));
    }

    #[test]
    fn removed_renderer_token_stays_literal() {
        let rendered = render_template_text("{{renderer.name}}", &TextRenderMetrics::default());
        assert_eq!(rendered, "{{renderer.name}}");
    }

    #[test]
    fn text_scale_uses_1080p_reference_frame() {
        assert!((text_reference_scale(1920.0, 1080.0) - 1.0).abs() < f32::EPSILON);
        assert!((text_reference_scale(1280.0, 720.0) - (2.0 / 3.0)).abs() < 0.0001);
        assert!((text_reference_scale(3840.0, 2160.0) - 2.0).abs() < 0.0001);
        assert!((text_reference_scale(1024.0, 1024.0) - (1024.0 / 1920.0)).abs() < 0.0001);
    }

    #[test]
    fn transparent_scene_background_skips_background_quad() {
        let config = TextSceneConfig {
            background_color: "transparent".into(),
            ..TextSceneConfig::default()
        };
        let layout = SceneLayout {
            scene: SceneConfig::Text(config.clone()),
            viewport_width: 320,
            viewport_height: 180,
            ..SceneLayout::default()
        };
        let mut scene = ProjectedScene::default();

        project_text_scene(&config, &layout, &TextRenderMetrics::default(), &mut scene);

        assert_eq!(scene.layer(SceneLayer::Background).len(), 0);
        assert!(!scene.layer(SceneLayer::Overlay).is_empty());
    }
}
