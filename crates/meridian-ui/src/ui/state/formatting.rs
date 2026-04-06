use std::path::Path;

use slint::{ModelRc, SharedString, VecModel};

use super::super::view::BarValue;

pub(super) fn format_view_range_label(seconds: f64) -> String {
    if seconds < 1.0 {
        format!("{:.0} ms", seconds * 1000.0)
    } else {
        format!("{} s", format_view_range_numeric(seconds))
    }
}

pub(super) fn format_view_range_numeric(seconds: f64) -> String {
    let mut text = format!("{seconds:.2}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

pub(super) fn file_name_or_full(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

// ── Formatting helpers ──

pub(super) fn format_number(n: impl Into<u64>) -> String {
    let n = n.into();
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub(super) fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub(super) fn format_duration(seconds: f64) -> String {
    if seconds >= 3600.0 {
        let h = (seconds / 3600.0).floor() as u64;
        let m = ((seconds % 3600.0) / 60.0).floor() as u64;
        let s = seconds % 60.0;
        format!("{}h {:02}m {:04.1}s", h, m, s)
    } else if seconds >= 60.0 {
        let m = (seconds / 60.0).floor() as u64;
        let s = seconds % 60.0;
        format!("{}m {:04.1}s", m, s)
    } else {
        format!("{:.3}s", seconds)
    }
}

pub(super) fn format_percentage(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

pub(super) fn format_percentage_compact(fraction: f64) -> String {
    let percent = fraction * 100.0;
    if percent >= 10.0 {
        format!("{:.1}%", percent)
    } else if percent >= 1.0 {
        format!("{:.2}%", percent)
    } else if percent > 0.0 {
        format!("<1%")
    } else {
        "0%".into()
    }
}

pub(super) fn format_duration_short(seconds: f64) -> String {
    if seconds >= 1.0 {
        format!("{:.2}s", seconds)
    } else if seconds >= 0.001 {
        format!("{:.1}ms", seconds * 1_000.0)
    } else {
        format!("{:.0}µs", seconds * 1_000_000.0)
    }
}

pub(super) fn midi_note_name(key: usize) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = key as i32 / 12 - 1;
    let name = NAMES[key % 12];
    format!("{}{}", name, octave)
}

pub(super) fn histogram_percentile(counts: &[u64], percentile: f64) -> Option<usize> {
    let total = counts.iter().sum::<u64>();
    if total == 0 {
        return None;
    }
    let target = ((total as f64 * percentile).ceil() as u64).max(1);
    let mut seen = 0u64;
    for (i, count) in counts.iter().copied().enumerate() {
        seen += count;
        if seen >= target {
            return Some(i);
        }
    }
    counts.len().checked_sub(1)
}

pub(super) fn build_velocity_profile_model(counts: &[u64]) -> ModelRc<BarValue> {
    let bucket_size = 16usize;
    let buckets: Vec<(String, u64)> = counts
        .chunks(bucket_size)
        .enumerate()
        .map(|(i, chunk)| {
            let start = i * bucket_size;
            let end = start + chunk.len().saturating_sub(1);
            let count = chunk.iter().sum();
            (format!("{}-{}", start, end), count)
        })
        .collect();
    let max = buckets
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let bars: Vec<BarValue> = buckets
        .into_iter()
        .map(|(label, count)| {
            let normalized = count as f32 / max;
            BarValue {
                value: normalized.sqrt(),
                label: SharedString::from(label),
                count: count as i32,
            }
        })
        .collect();
    ModelRc::from(std::rc::Rc::new(VecModel::from(bars)))
}

pub(super) fn build_bar_model_sparse(
    counts: &[u64],
    label_fn: impl Fn(usize) -> String,
    label_step: usize,
) -> ModelRc<BarValue> {
    let max = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
    let step = label_step.max(1);
    let last_index = counts.len().saturating_sub(1);
    let bars: Vec<BarValue> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| BarValue {
            value: c as f32 / max,
            label: SharedString::from(if i % step == 0 || i == last_index {
                label_fn(i)
            } else {
                String::new()
            }),
            count: c as i32,
        })
        .collect();
    ModelRc::from(std::rc::Rc::new(VecModel::from(bars)))
}

pub(super) fn build_key_bar_model(
    counts: &[u64],
    label_fn: impl Fn(usize) -> String,
) -> ModelRc<BarValue> {
    let default_visible_len = counts.len().min(128);
    let highest_nonzero = counts.iter().rposition(|&c| c > 0);
    let visible_len = highest_nonzero
        .map(|idx| (idx + 1).max(default_visible_len))
        .unwrap_or(default_visible_len)
        .max(1);
    build_bar_model(&counts[..visible_len.min(counts.len())], label_fn)
}

pub(super) fn build_bar_model(
    counts: &[u64],
    label_fn: impl Fn(usize) -> String,
) -> ModelRc<BarValue> {
    let max = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
    let bars: Vec<BarValue> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| BarValue {
            value: c as f32 / max,
            label: SharedString::from(label_fn(i)),
            count: c as i32,
        })
        .collect();
    ModelRc::from(std::rc::Rc::new(VecModel::from(bars)))
}
