use meridian_core::protocol::MidiAnalysisData;
use slint::{ModelRc, SharedString, VecModel};

use super::super::view::{App, BarValue, EventCount};
use super::formatting::{
    build_bar_model, build_bar_model_sparse, build_key_bar_model, build_velocity_profile_model,
    clamp_count_i32, format_bytes, format_duration, format_duration_short, format_number,
    format_percentage, format_percentage_compact, histogram_percentile, midi_note_name,
};

pub(super) fn apply_analysis_to_app(app: &App, analysis: &MidiAnalysisData) {
    // ── Overview ──
    app.set_analysis_note_count_text(format_number(analysis.total_notes).into());
    app.set_analysis_midi_length_text(format_duration(analysis.midi_length).into());

    let first_key = analysis
        .key_note_counts
        .iter()
        .position(|count| *count > 0)
        .unwrap_or(0);
    let last_key = analysis
        .key_note_counts
        .iter()
        .rposition(|count| *count > 0)
        .unwrap_or(0);
    app.set_analysis_key_range_text(
        format!(
            "{} – {} ({})",
            midi_note_name(first_key),
            midi_note_name(last_key),
            last_key - first_key + 1
        )
        .into(),
    );

    // ── Note density (computed from buckets) ──
    let bucket_width = if analysis.buckets.len() > 1 {
        analysis.buckets[1].time_seconds - analysis.buckets[0].time_seconds
    } else {
        analysis.midi_length.max(0.5)
    }
    .max(0.001);
    let peak_nps = analysis
        .buckets
        .iter()
        .map(|bucket| bucket.note_starts as f64 / bucket_width)
        .fold(0.0, f64::max);
    let avg_nps = if analysis.midi_length > 0.0 {
        analysis.total_notes as f64 / analysis.midi_length
    } else {
        analysis.total_notes as f64
    };
    app.set_analysis_note_density_text(
        format!("avg {:.1}/s peak {:.1}/s", avg_nps, peak_nps).into(),
    );
    app.set_analysis_peak_nps_text(format!("{:.1}", peak_nps).into());
    app.set_analysis_avg_nps_text(format!("{:.1}", avg_nps).into());

    // ── File metrics ──
    app.set_analysis_file_size_text(format_bytes(analysis.file.source_bytes).into());
    app.set_analysis_gzip_size_text(format_bytes(analysis.file.gzip_bytes).into());
    app.set_analysis_gzip_ratio_text(format!("{:.1}%", analysis.file.gzip_ratio * 100.0).into());
    app.set_analysis_format_text(format!("Type {}", analysis.file.format).into());
    app.set_analysis_declared_tracks_text(analysis.file.declared_track_count.to_string().into());
    app.set_analysis_actual_tracks_text(analysis.file.actual_track_count.to_string().into());
    app.set_analysis_track_count_text(analysis.file.actual_track_count.to_string().into());
    app.set_analysis_ticks_per_quarter_text(
        analysis
            .file
            .ticks_per_quarter
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into())
            .into(),
    );
    app.set_analysis_total_events_text(format_number(analysis.file.total_event_count).into());

    // ── Tempo metrics ──
    app.set_analysis_initial_bpm_text(format!("{:.1}", analysis.tempo.initial_bpm).into());
    app.set_analysis_min_bpm_text(format!("{:.1}", analysis.tempo.min_bpm).into());
    app.set_analysis_max_bpm_text(format!("{:.1}", analysis.tempo.max_bpm).into());
    app.set_analysis_avg_bpm_text(format!("{:.1}", analysis.tempo.avg_bpm_weighted_by_time).into());
    app.set_analysis_tempo_text(
        format!(
            "{:.0} BPM ({:.0}–{:.0})",
            analysis.tempo.avg_bpm_weighted_by_time, analysis.tempo.min_bpm, analysis.tempo.max_bpm
        )
        .into(),
    );
    app.set_analysis_time_signature_text("—".into()); // Not in analysis data

    // ── Note metrics ──
    app.set_analysis_avg_note_length_text(
        format_duration_short(analysis.notes.avg_note_length_seconds).into(),
    );
    app.set_analysis_min_note_length_text(
        format_duration_short(analysis.notes.min_note_length_seconds).into(),
    );
    app.set_analysis_max_note_length_text(
        format_duration_short(analysis.notes.max_note_length_seconds).into(),
    );
    app.set_analysis_total_note_duration_text(
        format_duration(analysis.notes.total_note_duration_seconds).into(),
    );
    app.set_analysis_max_polyphony_text(analysis.notes.max_simultaneous_notes.to_string().into());
    app.set_analysis_avg_polyphony_text(
        format!("{:.1}", analysis.notes.avg_simultaneous_notes).into(),
    );
    app.set_analysis_unique_onsets_text(format_number(analysis.notes.unique_onset_count).into());

    // ── Average velocity (computed from histogram) ──
    let total_velocity_notes: u64 = analysis.notes.velocity_note_on_counts.iter().sum();
    let weighted_velocity: u64 = analysis
        .notes
        .velocity_note_on_counts
        .iter()
        .enumerate()
        .map(|(i, &c)| i as u64 * c)
        .sum();
    let avg_velocity = if total_velocity_notes > 0 {
        weighted_velocity as f64 / total_velocity_notes as f64
    } else {
        0.0
    };
    if total_velocity_notes > 0 {
        let median_velocity =
            histogram_percentile(&analysis.notes.velocity_note_on_counts, 0.50).unwrap_or(0);
        let p90_velocity =
            histogram_percentile(&analysis.notes.velocity_note_on_counts, 0.90).unwrap_or(0);
        let low_velocity_notes: u64 = analysis.notes.velocity_note_on_counts.iter().take(33).sum();
        let high_velocity_notes: u64 = analysis.notes.velocity_note_on_counts.iter().skip(96).sum();

        app.set_analysis_avg_velocity_text(format!("{:.0}", avg_velocity).into());
        app.set_analysis_median_velocity_text(median_velocity.to_string().into());
        app.set_analysis_p90_velocity_text(p90_velocity.to_string().into());
        app.set_analysis_low_velocity_share_text(
            format_percentage(low_velocity_notes as f64 / total_velocity_notes as f64).into(),
        );
        app.set_analysis_high_velocity_share_text(
            format_percentage(high_velocity_notes as f64 / total_velocity_notes as f64).into(),
        );
    } else {
        app.set_analysis_avg_velocity_text("—".into());
        app.set_analysis_median_velocity_text("—".into());
        app.set_analysis_p90_velocity_text("—".into());
        app.set_analysis_low_velocity_share_text("—".into());
        app.set_analysis_high_velocity_share_text("—".into());
    }

    // ── Summary ──
    app.set_analysis_densest_key_text(midi_note_name(analysis.summary.densest_key).into());
    app.set_analysis_densest_key_notes_text(
        format_number(analysis.summary.densest_key_notes).into(),
    );

    // ── Histograms ──
    app.set_analysis_key_histogram(build_key_bar_model(&analysis.key_note_counts, |i| {
        midi_note_name(i)
    }));
    app.set_analysis_velocity_histogram(build_velocity_profile_model(
        &analysis.notes.velocity_note_on_counts,
    ));
    app.set_analysis_pitch_class_histogram(build_bar_model(
        &analysis.notes.pitch_class_note_counts,
        |i| {
            const NAMES: [&str; 12] = [
                "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
            ];
            NAMES.get(i).unwrap_or(&"?").to_string()
        },
    ));
    app.set_analysis_channel_histogram(build_bar_model(&analysis.notes.channel_note_counts, |i| {
        i.to_string()
    }));
    app.set_analysis_track_histogram(build_bar_model_sparse(
        &analysis.notes.track_note_counts,
        |i| format!("T{}", i),
        2,
    ));

    // ── Density timeline ──
    let peak_bucket = analysis
        .buckets
        .iter()
        .map(|b| b.note_starts)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let density_bars: Vec<BarValue> = analysis
        .buckets
        .iter()
        .map(|b| BarValue {
            value: b.note_starts as f32 / peak_bucket,
            label: SharedString::from(format!("{:.0}s", b.time_seconds)),
            count: clamp_count_i32(b.note_starts),
        })
        .collect();
    app.set_analysis_density_timeline(ModelRc::from(std::rc::Rc::new(VecModel::from(
        density_bars,
    ))));

    // ── Event breakdown ──
    let events = &analysis.events;
    let event_pairs: Vec<(&str, u64)> = vec![
        ("Note On", events.note_on_events),
        ("Note Off", events.note_off_events),
        ("Zero-Vel Note On", events.zero_velocity_note_on_events),
        ("Program Change", events.program_change_events),
        ("Control Change", events.control_change_events),
        ("Pitch Bend", events.pitch_bend_events),
        ("Channel Pressure", events.channel_pressure_events),
        ("Polyphonic Pressure", events.polyphonic_pressure_events),
        ("SysEx", events.sysex_events),
        ("Text", events.text_events),
        ("Lyric", events.lyric_events),
        ("Marker", events.marker_events),
        ("Cue Point", events.cue_point_events),
        ("Track Name", events.track_name_events),
        ("Instrument Name", events.instrument_name_events),
        ("Tempo", events.tempo_events),
        ("Time Signature", events.time_signature_events),
        ("Key Signature", events.key_signature_events),
    ];
    let total_accounted_events = event_pairs.iter().map(|(_, c)| *c).sum::<u64>().max(1);
    let event_counts: Vec<EventCount> = event_pairs
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| EventCount {
            name: SharedString::from(name),
            count: clamp_count_i32(count),
            fraction: count as f32 / total_accounted_events as f32,
            share_text: SharedString::from(format_percentage_compact(
                count as f64 / total_accounted_events as f64,
            )),
        })
        .collect();
    let split_index = event_counts.len().div_ceil(2);
    let (left_counts, right_counts) = event_counts.split_at(split_index);
    app.set_analysis_event_breakdown(ModelRc::from(std::rc::Rc::new(VecModel::from(
        event_counts.clone(),
    ))));
    app.set_analysis_event_breakdown_left(ModelRc::from(std::rc::Rc::new(VecModel::from(
        left_counts.to_vec(),
    ))));
    app.set_analysis_event_breakdown_right(ModelRc::from(std::rc::Rc::new(VecModel::from(
        right_counts.to_vec(),
    ))));

    let note_traffic_count =
        events.note_on_events + events.note_off_events + events.zero_velocity_note_on_events;
    let other_events_count = total_accounted_events.saturating_sub(note_traffic_count);
    let note_traffic_fraction = note_traffic_count as f32 / total_accounted_events as f32;
    let other_events_fraction = other_events_count as f32 / total_accounted_events as f32;
    app.set_analysis_note_traffic_count_text(format_number(note_traffic_count).into());
    app.set_analysis_note_traffic_share_text(
        format_percentage_compact(note_traffic_count as f64 / total_accounted_events as f64).into(),
    );
    app.set_analysis_note_traffic_fraction(note_traffic_fraction);
    app.set_analysis_other_events_count_text(format_number(other_events_count).into());
    app.set_analysis_other_events_share_text(
        format_percentage_compact(other_events_count as f64 / total_accounted_events as f64).into(),
    );
    app.set_analysis_other_events_fraction(other_events_fraction);

    let grouped_event_pairs: Vec<(&str, u64)> = vec![
        (
            "Controllers",
            events.control_change_events + events.program_change_events,
        ),
        (
            "Expressive",
            events.pitch_bend_events
                + events.channel_pressure_events
                + events.polyphonic_pressure_events,
        ),
        (
            "Meta text",
            events.text_events
                + events.lyric_events
                + events.marker_events
                + events.cue_point_events
                + events.track_name_events
                + events.instrument_name_events,
        ),
        (
            "Tempo/signature",
            events.tempo_events + events.time_signature_events + events.key_signature_events,
        ),
        ("SysEx", events.sysex_events),
    ];
    let non_note_total = grouped_event_pairs
        .iter()
        .map(|(_, count)| *count)
        .sum::<u64>()
        .max(1);
    if let Some((name, count)) = grouped_event_pairs
        .iter()
        .max_by_key(|(_, count)| *count)
        .copied()
    {
        app.set_analysis_top_non_note_family_text(name.into());
        app.set_analysis_top_non_note_share_text(
            format_percentage_compact(count as f64 / non_note_total as f64).into(),
        );
        app.set_analysis_top_non_note_fraction(count as f32 / non_note_total as f32);
    } else {
        app.set_analysis_top_non_note_family_text("—".into());
        app.set_analysis_top_non_note_share_text("—".into());
        app.set_analysis_top_non_note_fraction(0.0);
    }
    let event_composition: Vec<EventCount> = grouped_event_pairs
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| EventCount {
            name: SharedString::from(name),
            count: clamp_count_i32(count),
            fraction: count as f32 / non_note_total as f32,
            share_text: SharedString::from(format_percentage_compact(
                count as f64 / non_note_total as f64,
            )),
        })
        .collect();
    app.set_analysis_event_composition(ModelRc::from(std::rc::Rc::new(VecModel::from(
        event_composition,
    ))));
    app.set_analysis_event_total_text(format_number(analysis.file.total_event_count).into());
}
