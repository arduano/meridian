use rustc_hash::FxHashMap;

use crate::midi::MIDIAnalysisSummary;

use super::{
    AnalysisProgressUpdate,
    extract::{ExtractedAnalysis, NoteIntervalAggregate, TempoEventPoint, TickTransitionPoint},
    types::{
        CachedBucketDelta, CachedBucketStart, CachedMidiAnalysis, MidiAnalysisNoteMetrics,
        MidiAnalysisTempoMetrics,
    },
};

#[derive(Debug, Clone)]
struct TempoSegment {
    start_tick: u64,
    start_seconds: f64,
    micros_per_quarter: u32,
}

#[derive(Debug)]
struct TempoMap {
    segments: Vec<TempoSegment>,
    midi_length: f64,
    initial_bpm: f64,
    min_bpm: f64,
    max_bpm: f64,
    avg_bpm_weighted_by_time: f64,
}

pub(crate) fn reduce_extracted_analysis(
    extracted: ExtractedAnalysis,
    mut progress: impl FnMut(AnalysisProgressUpdate),
) -> Result<CachedMidiAnalysis, crate::error::MeridianError> {
    progress(AnalysisProgressUpdate {
        progress: 0.0,
        status: "Building Tempo Map".into(),
    });
    let tempo_map = build_tempo_map(extracted.ppq, extracted.max_tick, &extracted.tempo_events);
    progress(AnalysisProgressUpdate {
        progress: 0.1,
        status: "Reducing Note Onsets".into(),
    });

    let mut tick_seconds_cache = FxHashMap::<u64, f64>::default();
    let total_steps = (extracted.onset_counts.len()
        + extracted.tick_transitions.len()
        + extracted.intervals.len()
        + extracted.active_deltas.len())
    .max(1);
    let progress_stride = (total_steps / 200).max(1);
    let mut steps_completed = 0usize;

    let mut bucket_starts = Vec::with_capacity(extracted.onset_counts.len());
    let mut note_start_histogram = vec![0_u64; 257];
    let mut rolling_nps_peak = RollingNpsPeak::new();
    let mut unique_onset_count = 0_u64;
    for (tick, count) in &extracted.onset_counts {
        let time_seconds = seconds_for_tick(
            *tick,
            &tempo_map.segments,
            extracted.ppq,
            &mut tick_seconds_cache,
        );
        bucket_starts.push(CachedBucketStart {
            time_seconds,
            count: *count,
        });
        unique_onset_count += 1;
        let onset_size = (*count as usize).min(note_start_histogram.len() - 1);
        note_start_histogram[onset_size] += 1;
        rolling_nps_peak.push(time_seconds, *count);
        steps_completed += 1;
        if steps_completed == 1
            || steps_completed == total_steps
            || steps_completed.is_multiple_of(progress_stride)
        {
            progress(AnalysisProgressUpdate {
                progress: 0.1 + 0.2 * (steps_completed as f32 / total_steps as f32),
                status: "Reducing Note Onsets".into(),
            });
        }
    }

    let mut block_counts_per_key = vec![0usize; 256];
    let mut total_blocks = 0_u64;
    let mut keys_with_notes = 0usize;
    let mut max_blocks_per_key = 0usize;
    let mut max_notes_in_block = 0usize;
    for (_, key, count) in &extracted.block_starts {
        total_blocks += 1;
        let key_index = *key as usize;
        if block_counts_per_key[key_index] == 0 {
            keys_with_notes += 1;
        }
        block_counts_per_key[key_index] += 1;
        max_blocks_per_key = max_blocks_per_key.max(block_counts_per_key[key_index]);
        max_notes_in_block = max_notes_in_block.max(*count as usize);
    }

    let mut densest_key = 0usize;
    let mut densest_key_notes = 0u64;
    for (key, blocks) in block_counts_per_key.iter().enumerate() {
        if *blocks == 0 {
            continue;
        }
        let notes = extracted.key_note_counts[key];
        if notes > densest_key_notes {
            densest_key = key;
            densest_key_notes = notes;
        }
    }

    let mut bucket_active_deltas = Vec::with_capacity(extracted.active_deltas.len());
    progress(AnalysisProgressUpdate {
        progress: 0.3,
        status: "Preparing Bucket Summary".into(),
    });
    for (tick, delta) in &extracted.active_deltas {
        bucket_active_deltas.push(CachedBucketDelta {
            time_seconds: seconds_for_tick(
                *tick,
                &tempo_map.segments,
                extracted.ppq,
                &mut tick_seconds_cache,
            ),
            delta: *delta,
        });
        steps_completed += 1;
        if steps_completed == total_steps || steps_completed.is_multiple_of(progress_stride) {
            progress(AnalysisProgressUpdate {
                progress: 0.3 + 0.15 * (steps_completed as f32 / total_steps as f32),
                status: "Preparing Bucket Summary".into(),
            });
        }
    }

    progress(AnalysisProgressUpdate {
        progress: 0.45,
        status: "Reducing Polyphony".into(),
    });
    let (polyphony_area, max_simultaneous_notes) = reduce_polyphony(
        &extracted.tick_transitions,
        tempo_map.midi_length,
        &tempo_map.segments,
        extracted.ppq,
        &mut tick_seconds_cache,
        &mut steps_completed,
        total_steps,
        progress_stride,
        &mut progress,
        "Reducing Polyphony",
    );

    progress(AnalysisProgressUpdate {
        progress: 0.7,
        status: "Reducing Note Durations".into(),
    });
    let (total_note_duration_seconds, min_note_length_seconds, max_note_length_seconds) =
        reduce_durations(
            &extracted.intervals,
            &tempo_map.segments,
            extracted.ppq,
            &mut tick_seconds_cache,
            &mut steps_completed,
            total_steps,
            progress_stride,
            &mut progress,
            "Reducing Note Durations",
        );

    let avg_note_length_seconds = if extracted.total_notes > 0 {
        total_note_duration_seconds / extracted.total_notes as f64
    } else {
        0.0
    };
    let avg_simultaneous_notes = if tempo_map.midi_length > 0.0 {
        polyphony_area / tempo_map.midi_length
    } else {
        0.0
    };
    let notes_per_second_avg = if tempo_map.midi_length > 0.0 {
        extracted.total_notes as f64 / tempo_map.midi_length
    } else {
        extracted.total_notes as f64
    };
    let avg_notes_per_onset = if unique_onset_count > 0 {
        extracted.total_notes as f64 / unique_onset_count as f64
    } else {
        0.0
    };

    progress(AnalysisProgressUpdate {
        progress: 1.0,
        status: "Analysis Cache Complete".into(),
    });

    Ok(CachedMidiAnalysis::from_parts(
        tempo_map.midi_length,
        extracted.total_notes,
        extracted.total_event_count,
        extracted.actual_track_count,
        extracted.key_note_counts,
        MIDIAnalysisSummary {
            total_blocks,
            keys_with_notes,
            max_blocks_per_key,
            max_notes_in_block,
            densest_key,
            densest_key_notes,
        },
        extracted.event_metrics,
        MidiAnalysisNoteMetrics {
            pitch_class_note_counts: extracted.pitch_class_note_counts,
            velocity_note_on_counts: extracted.velocity_note_on_counts,
            track_note_counts: extracted.track_note_counts,
            channel_note_counts: extracted.channel_note_counts,
            track_channel_note_counts: extracted.track_channel_note_counts,
            note_start_histogram,
            total_note_duration_seconds,
            avg_note_length_seconds,
            min_note_length_seconds,
            max_note_length_seconds,
            max_simultaneous_notes,
            avg_simultaneous_notes,
            notes_per_second_peak: rolling_nps_peak.peak() as f64,
            notes_per_second_avg,
            unique_onset_count,
            avg_notes_per_onset,
        },
        MidiAnalysisTempoMetrics {
            initial_bpm: tempo_map.initial_bpm,
            min_bpm: tempo_map.min_bpm,
            max_bpm: tempo_map.max_bpm,
            avg_bpm_weighted_by_time: tempo_map.avg_bpm_weighted_by_time,
        },
        bucket_starts,
        bucket_active_deltas,
    ))
}

fn build_tempo_map(ppq: u16, max_tick: u64, tempo_events: &[TempoEventPoint]) -> TempoMap {
    let mut segments = vec![TempoSegment {
        start_tick: 0,
        start_seconds: 0.0,
        micros_per_quarter: 500_000,
    }];
    let mut current_tick = 0_u64;
    let mut current_seconds = 0.0;
    let mut current_micros = 500_000_u32;
    let mut current_bpm = 120.0_f64;
    let mut initial_bpm = 120.0_f64;
    let mut min_bpm = 120.0_f64;
    let mut max_bpm = 120.0_f64;
    let mut weighted_bpm_sum = 0.0;
    let mut last_tempo_seconds = 0.0;

    for event in tempo_events {
        if event.tick > current_tick {
            current_seconds +=
                tick_delta_to_seconds(event.tick - current_tick, current_micros, ppq);
            current_tick = event.tick;
        }

        let span = (current_seconds - last_tempo_seconds).max(0.0);
        weighted_bpm_sum += current_bpm * span;
        last_tempo_seconds = current_seconds;

        current_micros = event.micros_per_quarter.max(1);
        current_bpm = 60_000_000.0 / current_micros as f64;
        if current_seconds == 0.0 {
            initial_bpm = current_bpm;
        }
        min_bpm = min_bpm.min(current_bpm);
        max_bpm = max_bpm.max(current_bpm);

        if let Some(last_segment) = segments.last_mut() {
            if last_segment.start_tick == current_tick {
                last_segment.micros_per_quarter = current_micros;
            } else {
                segments.push(TempoSegment {
                    start_tick: current_tick,
                    start_seconds: current_seconds,
                    micros_per_quarter: current_micros,
                });
            }
        }
    }

    if max_tick > current_tick {
        current_seconds += tick_delta_to_seconds(max_tick - current_tick, current_micros, ppq);
    }
    weighted_bpm_sum += current_bpm * (current_seconds - last_tempo_seconds).max(0.0);

    TempoMap {
        segments,
        midi_length: current_seconds,
        initial_bpm,
        min_bpm,
        max_bpm,
        avg_bpm_weighted_by_time: if current_seconds > 0.0 {
            weighted_bpm_sum / current_seconds
        } else {
            current_bpm
        },
    }
}

fn seconds_for_tick(
    tick: u64,
    segments: &[TempoSegment],
    ppq: u16,
    cache: &mut FxHashMap<u64, f64>,
) -> f64 {
    if let Some(seconds) = cache.get(&tick) {
        return *seconds;
    }

    let index = match segments.binary_search_by_key(&tick, |segment| segment.start_tick) {
        Ok(index) => index,
        Err(index) => index.saturating_sub(1),
    };
    let segment = &segments[index];
    let seconds = segment.start_seconds
        + tick_delta_to_seconds(
            tick.saturating_sub(segment.start_tick),
            segment.micros_per_quarter,
            ppq,
        );
    cache.insert(tick, seconds);
    seconds
}

fn tick_delta_to_seconds(delta_ticks: u64, micros_per_quarter: u32, ppq: u16) -> f64 {
    delta_ticks as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq as f64
}

#[expect(
    clippy::too_many_arguments,
    reason = "The reduction pass threads shared progress and tempo-cache state through tight numeric helpers."
)]
fn reduce_polyphony(
    tick_transitions: &[TickTransitionPoint],
    midi_length: f64,
    segments: &[TempoSegment],
    ppq: u16,
    tick_seconds_cache: &mut FxHashMap<u64, f64>,
    steps_completed: &mut usize,
    total_steps: usize,
    progress_stride: usize,
    progress: &mut impl FnMut(AnalysisProgressUpdate),
    status: &str,
) -> (f64, u64) {
    let mut polyphony_area = 0.0;
    let mut max_simultaneous_notes = 0_u64;
    let mut active_notes = 0_i64;
    let mut last_time_seconds = 0.0;
    let mut index = 0usize;

    while index < tick_transitions.len() {
        let tick = tick_transitions[index].tick;
        let time_seconds = seconds_for_tick(tick, segments, ppq, tick_seconds_cache);
        polyphony_area += active_notes.max(0) as f64 * (time_seconds - last_time_seconds).max(0.0);

        let mut active_after_track = active_notes;
        while index < tick_transitions.len() && tick_transitions[index].tick == tick {
            let transition = &tick_transitions[index];
            max_simultaneous_notes = max_simultaneous_notes
                .max((active_after_track + transition.prefix_peak_delta).max(0) as u64);
            active_after_track += transition.net_delta;
            index += 1;
            *steps_completed += 1;
            if *steps_completed == total_steps || (*steps_completed).is_multiple_of(progress_stride)
            {
                progress(AnalysisProgressUpdate {
                    progress: 0.45 + 0.25 * (*steps_completed as f32 / total_steps as f32),
                    status: status.into(),
                });
            }
        }

        active_notes = active_after_track;
        last_time_seconds = time_seconds;
    }

    polyphony_area += active_notes.max(0) as f64 * (midi_length - last_time_seconds).max(0.0);
    (polyphony_area, max_simultaneous_notes)
}

#[expect(
    clippy::too_many_arguments,
    reason = "The duration reduction helper keeps progress accounting explicit alongside the cached tempo context."
)]
fn reduce_durations(
    intervals: &[NoteIntervalAggregate],
    segments: &[TempoSegment],
    ppq: u16,
    tick_seconds_cache: &mut FxHashMap<u64, f64>,
    steps_completed: &mut usize,
    total_steps: usize,
    progress_stride: usize,
    progress: &mut impl FnMut(AnalysisProgressUpdate),
    status: &str,
) -> (f64, f64, f64) {
    let mut total_note_duration_seconds = 0.0_f64;
    let mut min_note_length_seconds = f64::INFINITY;
    let mut max_note_length_seconds = 0.0_f64;

    for interval in intervals {
        let start_seconds =
            seconds_for_tick(interval.start_tick, segments, ppq, tick_seconds_cache);
        let end_seconds = seconds_for_tick(interval.end_tick, segments, ppq, tick_seconds_cache);
        let duration = (end_seconds - start_seconds).max(0.0);
        total_note_duration_seconds += duration * interval.count as f64;
        if interval.count > 0 {
            min_note_length_seconds = min_note_length_seconds.min(duration);
            max_note_length_seconds = max_note_length_seconds.max(duration);
        }
        *steps_completed += 1;
        if *steps_completed == total_steps || (*steps_completed).is_multiple_of(progress_stride) {
            progress(AnalysisProgressUpdate {
                progress: 0.7 + 0.3 * (*steps_completed as f32 / total_steps as f32),
                status: status.into(),
            });
        }
    }

    (
        total_note_duration_seconds,
        if min_note_length_seconds.is_finite() {
            min_note_length_seconds
        } else {
            0.0
        },
        max_note_length_seconds,
    )
}

struct RollingNpsPeak {
    bins: [u64; 1000],
    cursor_ms: i64,
    window_sum: u64,
    peak: u64,
}

impl RollingNpsPeak {
    fn new() -> Self {
        Self {
            bins: [0; 1000],
            cursor_ms: -1,
            window_sum: 0,
            peak: 0,
        }
    }

    fn push(&mut self, time_seconds: f64, count: u64) {
        let target_ms = (time_seconds.max(0.0) * 1000.0).floor() as i64;
        if self.cursor_ms < 0 {
            self.cursor_ms = target_ms;
        }

        if target_ms > self.cursor_ms {
            let advance = (target_ms - self.cursor_ms).min(1000);
            for step in 1..=advance {
                let index = ((self.cursor_ms + step) % 1000) as usize;
                self.window_sum = self.window_sum.saturating_sub(self.bins[index]);
                self.bins[index] = 0;
            }
            if target_ms - self.cursor_ms > 1000 {
                self.bins.fill(0);
                self.window_sum = 0;
            }
            self.cursor_ms = target_ms;
        }

        let index = (target_ms % 1000) as usize;
        self.bins[index] += count;
        self.window_sum += count;
        self.peak = self.peak.max(self.window_sum);
    }

    fn peak(&self) -> u64 {
        self.peak
    }
}
