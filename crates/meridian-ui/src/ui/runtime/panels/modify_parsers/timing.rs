use super::*;

// ── Full-note ↔ tick conversion helpers ─────────────────────────────
// UX policy: 1 full note = PPQ × 4 ticks (i.e. a semibreve / whole note).
// All user-facing timing fields are shown as decimal full notes.

pub(in super::super) fn get_ppq(app: &App) -> u32 {
    app.get_modify_source_ticks_per_quarter_text()
        .as_str()
        .trim()
        .parse::<u32>()
        .unwrap_or(480)
}

pub(in super::super) fn ticks_to_notes(ticks: u64, ppq: u32) -> f64 {
    ticks as f64 / (ppq as f64 * 4.0)
}

pub(in super::super) fn notes_to_ticks(notes: f64, ppq: u32) -> u64 {
    (notes * ppq as f64 * 4.0).round() as u64
}

/// Format a float trimming unnecessary trailing zeros.
pub(in super::super) fn format_decimal(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        if value >= 0.0 {
            format!("{}", value as u64)
        } else {
            format!("{}", value as i64)
        }
    } else {
        let s = format!("{:.8}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Convert a tick count to a full-notes string for display.
pub(in super::super) fn format_notes(ticks: u64, ppq: u32) -> String {
    format_decimal(ticks_to_notes(ticks, ppq))
}

/// Like `format_notes` but for `Option<T>` where T converts to u64.
pub(in super::super) fn format_option_notes<T: Into<u64>>(ticks: Option<T>, ppq: u32) -> String {
    ticks
        .map(|t| format_notes(t.into(), ppq))
        .unwrap_or_default()
}

/// Parse a user-entered full-notes string back to a tick count.
pub(in super::super) fn parse_notes(raw: &str, ppq: u32, label: &str) -> Result<u64, String> {
    let notes: f64 = parse_value(raw, label)?;
    Ok(notes_to_ticks(notes, ppq))
}

/// Parse an optional full-notes string (empty → None).
pub(in super::super) fn parse_optional_notes(
    raw: &str,
    ppq: u32,
    label: &str,
) -> Result<Option<u64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse_notes(trimmed, ppq, label).map(Some)
    }
}

/// Format tempo points with tick positions expressed as full notes.
pub(in super::super) fn format_tempo_points_notes(points: &[TempoPoint], ppq: u32) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", format_notes(point.tick, ppq), point.tempo))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse tempo points where tick positions are given as full notes.
pub(in super::super) fn parse_tempo_points_notes(
    raw: &str,
    ppq: u32,
) -> Result<Vec<TempoPoint>, String> {
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
pub(in super::super) fn format_time_warp_points_notes(
    points: &[TimeWarpPoint],
    ppq: u32,
) -> String {
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
pub(in super::super) fn parse_time_warp_points_notes(
    raw: &str,
    ppq: u32,
) -> Result<Vec<TimeWarpPoint>, String> {
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
