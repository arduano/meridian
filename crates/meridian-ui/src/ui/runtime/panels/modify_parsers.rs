use super::*;

pub(super) fn selected_modify_pass_key(app: &App) -> String {
    match app.get_modify_pass_key_text().as_str() {
        "" | "custom" => "quantize".to_string(),
        key => key.to_string(),
    }
}

pub(super) fn serialize_modify_config(config: &MidiFileProcessingConfig) -> String {
    serde_json::to_string_pretty(config).unwrap_or_else(|_| "{\n  \"tool\": null\n}".into())
}

pub(super) fn parse_modify_config_text(raw: &str) -> Result<MidiFileProcessingConfig, String> {
    if raw.trim().is_empty() {
        return Err("modify config is empty".into());
    }

    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("invalid JSON: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("invalid process config: {error}"))
}

pub(super) fn fallback_modify_config(app: &App) -> MidiFileProcessingConfig {
    parse_modify_config_text(app.get_modify_config_text().as_str())
        .or_else(|_| parse_modify_config_text(app.get_modify_last_valid_config_text().as_str()))
        .unwrap_or_else(|_| default_modify_config(selected_modify_pass_key(app).as_str()))
}

pub(super) fn set_modify_config(app: &App, config: &MidiFileProcessingConfig) {
    app.set_modify_config_text(serialize_modify_config(config).into());
    validate_modify_config(app);
}

pub(super) fn option_text<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

pub(super) fn toggle_text(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

pub(super) fn parse_value<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Display,
{
    raw.trim()
        .parse::<T>()
        .map_err(|error| format!("invalid {label}: {error}"))
}

pub(super) fn parse_optional_value<T>(raw: &str, label: &str) -> Result<Option<T>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse_value(trimmed, label).map(Some)
    }
}

pub(super) fn parse_bool_toggle(raw: &str, label: &str) -> Result<bool, String> {
    match raw.trim() {
        "on" | "yes" | "true" | "1" => Ok(true),
        "off" | "no" | "false" | "0" => Ok(false),
        other => Err(format!("invalid {label}: {other}")),
    }
}

pub(super) fn parse_serde_enum<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    serde_json::from_str::<T>(&format!("\"{trimmed}\""))
        .map_err(|_| format!("invalid {label}: {trimmed}"))
}

pub(super) fn format_serde_enum<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"\"".into())
        .trim_matches('"')
        .to_string()
}

// ── Full-note ↔ tick conversion helpers ─────────────────────────────
// UX policy: 1 full note = PPQ × 4 ticks (i.e. a semibreve / whole note).
// All user-facing timing fields are shown as decimal full notes.

pub(super) fn get_ppq(app: &App) -> u32 {
    app.get_analysis_ticks_per_quarter_text()
        .as_str()
        .trim()
        .parse::<u32>()
        .unwrap_or(480)
}

pub(super) fn ticks_to_notes(ticks: u64, ppq: u32) -> f64 {
    ticks as f64 / (ppq as f64 * 4.0)
}

pub(super) fn notes_to_ticks(notes: f64, ppq: u32) -> u64 {
    (notes * ppq as f64 * 4.0).round() as u64
}

/// Format a float trimming unnecessary trailing zeros.
pub(super) fn format_decimal(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        if value >= 0.0 {
            format!("{}", value as u64)
        } else {
            format!("{}", value as i64)
        }
    } else {
        let s = format!("{:.8}", value);
        s.trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Convert a tick count to a full-notes string for display.
pub(super) fn format_notes(ticks: u64, ppq: u32) -> String {
    format_decimal(ticks_to_notes(ticks, ppq))
}

/// Like `format_notes` but for `Option<T>` where T converts to u64.
pub(super) fn format_option_notes<T: Into<u64>>(ticks: Option<T>, ppq: u32) -> String {
    ticks
        .map(|t| format_notes(t.into(), ppq))
        .unwrap_or_default()
}

/// Parse a user-entered full-notes string back to a tick count.
pub(super) fn parse_notes(raw: &str, ppq: u32, label: &str) -> Result<u64, String> {
    let notes: f64 = parse_value(raw, label)?;
    Ok(notes_to_ticks(notes, ppq))
}

/// Parse an optional full-notes string (empty → None).
pub(super) fn parse_optional_notes(raw: &str, ppq: u32, label: &str) -> Result<Option<u64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse_notes(trimmed, ppq, label).map(Some)
    }
}

/// Format tempo points with tick positions expressed as full notes.
pub(super) fn format_tempo_points_notes(points: &[TempoPoint], ppq: u32) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", format_notes(point.tick, ppq), point.tempo))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse tempo points where tick positions are given as full notes.
pub(super) fn parse_tempo_points_notes(raw: &str, ppq: u32) -> Result<Vec<TempoPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((pos, tempo)) = entry.split_once(':') else {
            return Err(format!("invalid tempo point: {entry}"));
        };
        points.push(TempoPoint {
            tick: parse_notes(pos.trim(), ppq, "tempo position")?,
            tempo: parse_value(tempo.trim(), "tempo value")?,
        });
    }
    Ok(points)
}

/// Format time-warp points with both ticks expressed as full notes.
pub(super) fn format_time_warp_points_notes(points: &[TimeWarpPoint], ppq: u32) -> String {
    points
        .iter()
        .map(|point| {
            format!(
                "{}->{}",
                format_notes(point.source_tick, ppq),
                format_notes(point.dest_tick, ppq)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse time-warp points where both positions are given as full notes.
pub(super) fn parse_time_warp_points_notes(raw: &str, ppq: u32) -> Result<Vec<TimeWarpPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((from, to)) = entry.split_once("->") else {
            return Err(format!("invalid time-warp point: {entry}"));
        };
        points.push(TimeWarpPoint {
            source_tick: parse_notes(from.trim(), ppq, "source position")?,
            dest_tick: parse_notes(to.trim(), ppq, "dest position")?,
        });
    }
    Ok(points)
}

pub(super) fn csv_parts<'a>(raw: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

pub(super) fn parse_u8_token(raw: &str, label: &str) -> Result<u8, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|error| format!("invalid {label}: {error}"))
    } else {
        parse_value(trimmed, label)
    }
}

pub(super) fn parse_arrow_entries<T>(raw: &str, label: &str) -> Result<Vec<(T, T)>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let mut result = Vec::new();
    for entry in csv_parts(raw) {
        let Some((from, to)) = entry.split_once("->") else {
            return Err(format!("invalid {label} entry: {entry}"));
        };
        result.push((
            parse_value(from.trim(), label)?,
            parse_value(to.trim(), label)?,
        ));
    }
    Ok(result)
}

pub(super) fn parse_tempo_points(raw: &str) -> Result<Vec<TempoPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((tick, tempo)) = entry.split_once(':') else {
            return Err(format!("invalid tempo point: {entry}"));
        };
        points.push(TempoPoint {
            tick: parse_value(tick.trim(), "tempo tick")?,
            tempo: parse_value(tempo.trim(), "tempo value")?,
        });
    }
    Ok(points)
}

pub(super) fn format_tempo_points(points: &[TempoPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", point.tick, point.tempo))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_time_warp_points(raw: &str) -> Result<Vec<TimeWarpPoint>, String> {
    let mut points = Vec::new();
    for (source_tick, dest_tick) in parse_arrow_entries::<u64>(raw, "time-warp point")? {
        points.push(TimeWarpPoint {
            source_tick,
            dest_tick,
        });
    }
    Ok(points)
}

pub(super) fn format_time_warp_points(points: &[TimeWarpPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}->{}", point.source_tick, point.dest_tick))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_channel_mappings(raw: &str) -> Result<Vec<ChannelMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "channel mapping")? {
        mappings.push(ChannelMapEntry { from, to });
    }
    Ok(mappings)
}

pub(super) fn format_channel_mappings(mappings: &[ChannelMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_track_mappings(raw: &str) -> Result<Vec<TrackMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<usize>(raw, "track mapping")? {
        mappings.push(TrackMapEntry { from, to });
    }
    Ok(mappings)
}

pub(super) fn format_track_mappings(mappings: &[TrackMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_channel_programs(raw: &str) -> Result<Vec<ChannelProgram>, String> {
    let mut programs = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, program)) = entry.split_once(':') else {
            return Err(format!("invalid startup program: {entry}"));
        };
        programs.push(ChannelProgram {
            channel: parse_value(channel.trim(), "startup channel")?,
            program: parse_value(program.trim(), "startup program")?,
        });
    }
    Ok(programs)
}

pub(super) fn format_channel_programs(programs: &[ChannelProgram]) -> String {
    programs
        .iter()
        .map(|program| format!("{}:{}", program.channel, program.program))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_u8_list(raw: &str, label: &str) -> Result<Vec<u8>, String> {
    csv_parts(raw)
        .map(|entry| parse_value(entry, label))
        .collect()
}

pub(super) fn format_u8_list(values: &[u8]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_controller_mappings(raw: &str) -> Result<Vec<ControllerMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "controller mapping")? {
        mappings.push(ControllerMapEntry { from, to });
    }
    Ok(mappings)
}

pub(super) fn format_controller_mappings(mappings: &[ControllerMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_controller_scales(raw: &str) -> Result<Vec<ControllerScaleEntry>, String> {
    let mut entries = Vec::new();
    for entry in csv_parts(raw) {
        let Some((controller, scale)) = entry.split_once(':') else {
            return Err(format!("invalid controller scale: {entry}"));
        };
        entries.push(ControllerScaleEntry {
            controller: parse_value(controller.trim(), "scale controller")?,
            scale: parse_value(scale.trim(), "scale factor")?,
        });
    }
    Ok(entries)
}

pub(super) fn format_controller_scales(entries: &[ControllerScaleEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.controller, entry.scale))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_control_values(raw: &str) -> Result<Vec<ControlValue>, String> {
    let mut values = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, controller_value)) = entry.split_once(':') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        let Some((controller, value)) = controller_value.split_once('=') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        values.push(ControlValue {
            channel: parse_value(channel.trim(), "inject channel")?,
            controller: parse_value(controller.trim(), "inject controller")?,
            value: parse_value(value.trim(), "inject value")?,
        });
    }
    Ok(values)
}

pub(super) fn format_control_values(values: &[ControlValue]) -> String {
    values
        .iter()
        .map(|value| format!("{}:{}={}", value.channel, value.controller, value.value))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_velocity_points(raw: &str) -> Result<Vec<VelocityPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((input, output)) = entry.split_once(':') else {
            return Err(format!("invalid velocity point: {entry}"));
        };
        points.push(VelocityPoint {
            input: parse_value(input.trim(), "velocity input")?,
            output: parse_value(output.trim(), "velocity output")?,
        });
    }
    Ok(points)
}

pub(super) fn format_velocity_points(points: &[VelocityPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", point.input, point.output))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_key_mappings(raw: &str) -> Result<Vec<KeyMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "key mapping")? {
        mappings.push(KeyMapEntry { from, to });
    }
    Ok(mappings)
}

pub(super) fn format_key_mappings(mappings: &[KeyMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_key_range(raw: &str) -> Result<Option<KeyRange>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Some((min, max)) = trimmed.split_once('-') else {
        return Err(format!("invalid fold range: {trimmed}"));
    };
    Ok(Some(KeyRange {
        min: parse_value(min.trim(), "fold range min")?,
        max: parse_value(max.trim(), "fold range max")?,
    }))
}

pub(super) fn format_key_range(range: &Option<KeyRange>) -> String {
    range
        .as_ref()
        .map(|range| format!("{}-{}", range.min, range.max))
        .unwrap_or_default()
}

pub(super) fn parse_text_kinds(raw: &str) -> Result<Vec<TextKind>, String> {
    csv_parts(raw)
        .map(|entry| parse_serde_enum(entry, "text kind"))
        .collect()
}

pub(super) fn format_text_kinds(kinds: &[TextKind]) -> String {
    kinds
        .iter()
        .map(format_serde_enum)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn parse_sysex_messages(raw: &str) -> Result<Vec<Vec<u8>>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut messages = Vec::new();
    for message in trimmed
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let mut bytes = Vec::new();
        for token in message
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            bytes.push(parse_u8_token(token, "sysex byte")?);
        }
        messages.push(bytes);
    }
    Ok(messages)
}

pub(super) fn format_sysex_messages(messages: &[Vec<u8>]) -> String {
    messages
        .iter()
        .map(|message| {
            message
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
