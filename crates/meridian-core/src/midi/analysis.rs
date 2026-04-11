mod accumulator;
mod file_metrics;
mod types;

use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use midi_toolkit::events::{Event, MIDIEventEnum, TextEventKind};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::midi::{
    MIDIAnalysisSummary, display_cache::DisplayMidiCache, parsed::ParsedMidiFile,
    parsed::ToolkitMidiFile,
};

pub(crate) use accumulator::MidiAnalysisAccumulator;
pub use file_metrics::{
    analyze_file_metrics, analyze_file_metrics_with_values, gzip_size_for_path,
};
use types::{CachedBucketDelta, CachedBucketStart};
pub use types::{
    CachedMidiAnalysis, MidiAnalysisBucket, MidiAnalysisData, MidiAnalysisEventMetrics,
    MidiAnalysisFileMetrics, MidiAnalysisKind, MidiAnalysisNoteMetrics, MidiAnalysisTempoMetrics,
};

#[derive(Debug, Clone)]
pub struct AnalysisProgressUpdate {
    pub progress: f32,
    pub status: String,
}

#[derive(Debug, Clone)]
struct TempoEventPoint {
    tick: u64,
    track: u32,
    event_index: u32,
    micros_per_quarter: u32,
}

#[derive(Debug, Clone)]
struct TickTransitionPoint {
    tick: u64,
    track: u32,
    net_delta: i64,
    prefix_peak_delta: i64,
}

#[derive(Debug, Clone)]
struct NoteIntervalAggregate {
    start_tick: u64,
    end_tick: u64,
    count: u64,
}

#[derive(Debug, Clone)]
struct TrackAnalysisPartial {
    track_index: u32,
    total_event_count: u64,
    total_notes: u64,
    max_tick: u64,
    event_metrics: MidiAnalysisEventMetrics,
    velocity_note_on_counts: [u64; 128],
    key_note_counts: [u64; 256],
    pitch_class_note_counts: [u64; 12],
    channel_note_counts: [u64; 16],
    track_channel_note_counts: [u64; 16],
    tempo_events: Vec<TempoEventPoint>,
    onset_counts: Vec<(u64, u64)>,
    block_starts: Vec<(u64, u8, u64)>,
    tick_transitions: Vec<TickTransitionPoint>,
    intervals: Vec<NoteIntervalAggregate>,
    unresolved_starts: Vec<(u64, u64)>,
}

#[derive(Debug)]
struct ExtractedAnalysis {
    ppq: u16,
    actual_track_count: usize,
    total_event_count: u64,
    total_notes: u64,
    max_tick: u64,
    event_metrics: MidiAnalysisEventMetrics,
    velocity_note_on_counts: Vec<u64>,
    key_note_counts: Vec<u64>,
    pitch_class_note_counts: Vec<u64>,
    track_note_counts: Vec<u64>,
    channel_note_counts: Vec<u64>,
    track_channel_note_counts: Vec<u64>,
    tempo_events: Vec<TempoEventPoint>,
    onset_counts: Vec<(u64, u64)>,
    block_starts: Vec<(u64, u8, u64)>,
    tick_transitions: Vec<TickTransitionPoint>,
    active_deltas: Vec<(u64, i64)>,
    intervals: Vec<NoteIntervalAggregate>,
}

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

pub fn build_cached_midi_analysis_with_progress(
    parsed: &ParsedMidiFile,
    progress: impl FnMut(f32) + Send,
) -> Result<CachedMidiAnalysis, crate::error::MeridianError> {
    let progress = Mutex::new(progress);
    build_cached_midi_analysis_with_detailed_progress(parsed, |update| {
        if let Ok(mut callback) = progress.lock() {
            (*callback)(update.progress);
        }
    })
}

pub fn build_cached_midi_analysis_with_detailed_progress(
    parsed: &ParsedMidiFile,
    progress: impl Fn(AnalysisProgressUpdate) + Sync + Send,
) -> Result<CachedMidiAnalysis, crate::error::MeridianError> {
    progress(AnalysisProgressUpdate {
        progress: 0.0,
        status: "Preparing Analysis".into(),
    });
    let extracted = extract_analysis(parsed, &progress)?;
    progress(AnalysisProgressUpdate {
        progress: 0.65,
        status: "Merging Extracted Analysis".into(),
    });
    reduce_extracted_analysis(extracted, |update| {
        progress(AnalysisProgressUpdate {
            progress: 0.65 + update.progress * 0.35,
            status: update.status,
        });
    })
}

pub fn build_buckets_from_parsed_with_progress(
    parsed: &ParsedMidiFile,
    bucket_count: usize,
    _midi_length: f64,
    mut progress: impl FnMut(f32) + Send,
) -> Result<Vec<MidiAnalysisBucket>, crate::error::MeridianError> {
    progress(0.0);
    let cached = build_cached_midi_analysis_with_progress(parsed, |value| progress(value * 0.9))?;
    let buckets = cached.build_buckets_from_note_spans(bucket_count.clamp(1, 8192));
    progress(1.0);
    Ok(buckets)
}

pub fn analyze_cached_midi(
    parsed: &ParsedMidiFile,
    cached: &CachedMidiAnalysis,
    buckets: Vec<MidiAnalysisBucket>,
) -> MidiAnalysisData {
    analyze_cached_midi_with_gzip_bytes(parsed, cached, buckets, None)
}

pub fn analyze_cached_midi_with_gzip_bytes(
    parsed: &ParsedMidiFile,
    cached: &CachedMidiAnalysis,
    buckets: Vec<MidiAnalysisBucket>,
    gzip_bytes: Option<u64>,
) -> MidiAnalysisData {
    MidiAnalysisData {
        midi_length: cached.midi_length(),
        total_notes: cached.total_notes(),
        key_note_counts: cached.key_note_counts().to_vec(),
        summary: cached.summary(),
        buckets,
        file: analyze_file_metrics_with_values(
            parsed,
            cached.actual_track_count(),
            cached.total_event_count(),
            gzip_bytes,
        ),
        events: cached.events().clone(),
        notes: cached.notes().clone(),
        tempo: cached.tempo().clone(),
    }
}

pub fn analyze_midi(
    parsed: &ParsedMidiFile,
    cache: &DisplayMidiCache,
    cached: &CachedMidiAnalysis,
    bucket_count: usize,
) -> MidiAnalysisData {
    let bucket_count = bucket_count.clamp(1, 8192);
    analyze_cached_midi(
        parsed,
        cached,
        cached.build_buckets(cache, bucket_count, cached.midi_length()),
    )
}

pub fn analyze_parsed_midi_with_progress(
    parsed: &ParsedMidiFile,
    cached: &CachedMidiAnalysis,
    bucket_count: usize,
    mut progress: impl FnMut(f32),
) -> Result<MidiAnalysisData, crate::error::MeridianError> {
    progress(0.0);
    let buckets = cached.build_buckets_from_note_spans(bucket_count.clamp(1, 8192));
    progress(1.0);
    Ok(analyze_cached_midi(parsed, cached, buckets))
}

pub fn select_analysis_kinds(
    mut analysis: MidiAnalysisData,
    kinds: &[MidiAnalysisKind],
) -> MidiAnalysisData {
    let has = |kind| kinds.is_empty() || kinds.contains(&kind);
    if !has(MidiAnalysisKind::File) {
        analysis.file = MidiAnalysisFileMetrics::default();
    }
    if !has(MidiAnalysisKind::Summary) {
        analysis.key_note_counts.clear();
        analysis.summary = MIDIAnalysisSummary {
            total_blocks: 0,
            keys_with_notes: 0,
            max_blocks_per_key: 0,
            max_notes_in_block: 0,
            densest_key: 0,
            densest_key_notes: 0,
        };
    }
    if !has(MidiAnalysisKind::Events) {
        analysis.events = MidiAnalysisEventMetrics::default();
    }
    if !has(MidiAnalysisKind::Notes) {
        analysis.notes = MidiAnalysisNoteMetrics::default();
    }
    if !has(MidiAnalysisKind::Tempo) {
        analysis.tempo = MidiAnalysisTempoMetrics::default();
    }
    if !has(MidiAnalysisKind::Buckets) {
        analysis.buckets.clear();
    }
    analysis
}

fn extract_analysis(
    parsed: &ParsedMidiFile,
    progress: &(impl Fn(AnalysisProgressUpdate) + Sync),
) -> Result<ExtractedAnalysis, crate::error::MeridianError> {
    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(crate::error::MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let raw_track_count = midi.track_count();
    let actual_track_count = raw_track_count.max(1);
    progress(AnalysisProgressUpdate {
        progress: 0.0,
        status: format!("Extracting Tracks 0/{actual_track_count}"),
    });
    let finished_tracks = AtomicUsize::new(0);
    let reported_percent = AtomicUsize::new(0);
    let partials = (0..raw_track_count)
        .into_par_iter()
        .map(|track_index| {
            let result = extract_track_analysis(midi, track_index as u32);
            let finished = finished_tracks.fetch_add(1, Ordering::Relaxed) + 1;
            let percent = finished * 100 / actual_track_count;
            let mut last = reported_percent.load(Ordering::Relaxed);
            while percent > last {
                match reported_percent.compare_exchange(
                    last,
                    percent,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        progress(AnalysisProgressUpdate {
                            progress: (finished as f32 / actual_track_count as f32).clamp(0.0, 1.0),
                            status: format!("Extracting Tracks {finished}/{actual_track_count}"),
                        });
                        break;
                    }
                    Err(updated) => last = updated,
                }
            }
            result
        })
        .collect::<Vec<_>>();

    let mut resolved_partials = Vec::with_capacity(partials.len());
    let mut max_tick = 0_u64;
    for partial in partials {
        let partial = partial?;
        max_tick = max_tick.max(partial.max_tick);
        resolved_partials.push(partial);
    }

    let mut total_event_count = 0_u64;
    let mut total_notes = 0_u64;
    let mut event_metrics = MidiAnalysisEventMetrics::default();
    let mut velocity_note_on_counts = vec![0_u64; 128];
    let mut key_note_counts = vec![0_u64; 256];
    let mut pitch_class_note_counts = vec![0_u64; 12];
    let mut track_note_counts = vec![0_u64; actual_track_count];
    let mut channel_note_counts = vec![0_u64; 16];
    let mut track_channel_note_counts = vec![0_u64; actual_track_count * 16];
    let mut tempo_events = Vec::new();
    let mut onset_counts = FxHashMap::<u64, u64>::default();
    let mut block_starts = FxHashMap::<(u64, u8), u64>::default();
    let mut active_deltas = FxHashMap::<u64, i64>::default();
    let mut intervals = FxHashMap::<(u64, u64), u64>::default();
    let mut tick_transitions = Vec::new();

    for partial in resolved_partials {
        total_event_count += partial.total_event_count;
        total_notes += partial.total_notes;
        accumulate_event_metrics(&mut event_metrics, &partial.event_metrics);
        for (index, count) in partial.velocity_note_on_counts.into_iter().enumerate() {
            velocity_note_on_counts[index] += count;
        }
        for (index, count) in partial.key_note_counts.into_iter().enumerate() {
            key_note_counts[index] += count;
        }
        for (index, count) in partial.pitch_class_note_counts.into_iter().enumerate() {
            pitch_class_note_counts[index] += count;
        }
        let track_index = partial.track_index as usize;
        if let Some(slot) = track_note_counts.get_mut(track_index) {
            *slot += partial.total_notes;
        }
        for (index, count) in partial.channel_note_counts.into_iter().enumerate() {
            channel_note_counts[index] += count;
        }
        for (index, count) in partial.track_channel_note_counts.into_iter().enumerate() {
            let target = track_index * 16 + index;
            if let Some(slot) = track_channel_note_counts.get_mut(target) {
                *slot += count;
            }
        }
        tempo_events.extend(partial.tempo_events);
        for (tick, count) in partial.onset_counts {
            *onset_counts.entry(tick).or_insert(0) += count;
        }
        for (tick, key, count) in partial.block_starts {
            *block_starts.entry((tick, key)).or_insert(0) += count;
        }
        for transition in partial.tick_transitions {
            *active_deltas.entry(transition.tick).or_insert(0) += transition.net_delta;
            tick_transitions.push(transition);
        }
        for interval in partial.intervals {
            *intervals
                .entry((interval.start_tick, interval.end_tick))
                .or_insert(0) += interval.count;
        }
        for (start_tick, count) in partial.unresolved_starts {
            *active_deltas.entry(max_tick).or_insert(0) -= count as i64;
            *intervals.entry((start_tick, max_tick)).or_insert(0) += count;
        }
    }

    tempo_events.par_sort_unstable_by_key(|event| (event.tick, event.track, event.event_index));
    tick_transitions.par_sort_unstable_by_key(|transition| (transition.tick, transition.track));

    let mut onset_counts = onset_counts.into_iter().collect::<Vec<_>>();
    onset_counts.par_sort_unstable_by_key(|(tick, _)| *tick);

    let mut block_starts = block_starts
        .into_iter()
        .map(|((tick, key), count)| (tick, key, count))
        .collect::<Vec<_>>();
    block_starts.par_sort_unstable_by_key(|(tick, key, _)| (*tick, *key));

    let mut active_deltas = active_deltas
        .into_iter()
        .filter(|(_, delta)| *delta != 0)
        .collect::<Vec<_>>();
    active_deltas.par_sort_unstable_by_key(|(tick, _)| *tick);

    let mut intervals = intervals
        .into_iter()
        .map(|((start_tick, end_tick), count)| NoteIntervalAggregate {
            start_tick,
            end_tick,
            count,
        })
        .collect::<Vec<_>>();
    intervals.par_sort_unstable_by_key(|interval| (interval.start_tick, interval.end_tick));

    Ok(ExtractedAnalysis {
        ppq,
        actual_track_count,
        total_event_count,
        total_notes,
        max_tick,
        event_metrics,
        velocity_note_on_counts,
        key_note_counts,
        pitch_class_note_counts,
        track_note_counts,
        channel_note_counts,
        track_channel_note_counts,
        tempo_events,
        onset_counts,
        block_starts,
        tick_transitions,
        active_deltas,
        intervals,
    })
}

fn extract_track_analysis(
    midi: &ToolkitMidiFile,
    track_index: u32,
) -> Result<TrackAnalysisPartial, crate::error::MeridianError> {
    let mut absolute_tick = 0_u64;
    let mut event_index = 0_u32;
    let mut total_event_count = 0_u64;
    let mut total_notes = 0_u64;
    let mut max_tick = 0_u64;
    let mut event_metrics = MidiAnalysisEventMetrics::default();
    let mut velocity_note_on_counts = [0_u64; 128];
    let mut key_note_counts = [0_u64; 256];
    let mut pitch_class_note_counts = [0_u64; 12];
    let mut channel_note_counts = [0_u64; 16];
    let mut track_channel_note_counts = [0_u64; 16];
    let mut tempo_events = Vec::new();
    let mut onset_counts = FxHashMap::<u64, u64>::default();
    let mut block_starts = FxHashMap::<(u64, u8), u64>::default();
    let mut interval_counts = FxHashMap::<(u64, u64), u64>::default();
    let mut unresolved_starts = FxHashMap::<u64, u64>::default();
    let mut open_notes = FxHashMap::<(u8, u8), VecDeque<u64>>::default();
    let mut tick_transitions = Vec::new();
    let mut current_transition_tick = None;
    let mut current_transition_net = 0_i64;
    let mut current_transition_peak = 0_i64;
    let mut current_transition_running = 0_i64;

    let track = midi.iter_track(track_index).ok_or_else(|| {
        crate::error::MeridianError::InvalidMidi(format!("missing track {}", track_index))
    })?;

    for event in track {
        let event = event.map_err(|e| crate::error::MeridianError::MidiLoad(format!("{e:?}")))?;
        absolute_tick = absolute_tick.saturating_add(event.delta);
        max_tick = max_tick.max(absolute_tick);
        total_event_count += 1;

        match event.as_event() {
            Event::NoteOn(note_on) => {
                event_metrics.note_on_events += 1;
                velocity_note_on_counts[note_on.velocity as usize] += 1;
                if note_on.velocity == 0 {
                    event_metrics.zero_velocity_note_on_events += 1;
                    if close_note_interval(
                        &mut open_notes,
                        &mut interval_counts,
                        note_on.key,
                        note_on.channel,
                        absolute_tick,
                    ) {
                        record_tick_transition(
                            &mut tick_transitions,
                            &mut current_transition_tick,
                            &mut current_transition_net,
                            &mut current_transition_peak,
                            &mut current_transition_running,
                            absolute_tick,
                            -1,
                            track_index,
                        );
                    }
                } else {
                    total_notes += 1;
                    key_note_counts[note_on.key as usize] += 1;
                    pitch_class_note_counts[note_on.key as usize % 12] += 1;
                    channel_note_counts[note_on.channel as usize] += 1;
                    track_channel_note_counts[note_on.channel as usize] += 1;
                    *onset_counts.entry(absolute_tick).or_insert(0) += 1;
                    *block_starts
                        .entry((absolute_tick, note_on.key))
                        .or_insert(0) += 1;
                    open_notes
                        .entry((note_on.key, note_on.channel))
                        .or_default()
                        .push_back(absolute_tick);
                    record_tick_transition(
                        &mut tick_transitions,
                        &mut current_transition_tick,
                        &mut current_transition_net,
                        &mut current_transition_peak,
                        &mut current_transition_running,
                        absolute_tick,
                        1,
                        track_index,
                    );
                }
            }
            Event::NoteOff(note_off) => {
                event_metrics.note_off_events += 1;
                if close_note_interval(
                    &mut open_notes,
                    &mut interval_counts,
                    note_off.key,
                    note_off.channel,
                    absolute_tick,
                ) {
                    record_tick_transition(
                        &mut tick_transitions,
                        &mut current_transition_tick,
                        &mut current_transition_net,
                        &mut current_transition_peak,
                        &mut current_transition_running,
                        absolute_tick,
                        -1,
                        track_index,
                    );
                }
            }
            Event::ProgramChange(_) => event_metrics.program_change_events += 1,
            Event::ControlChange(_) => event_metrics.control_change_events += 1,
            Event::PitchWheelChange(_) => event_metrics.pitch_bend_events += 1,
            Event::ChannelPressure(_) => event_metrics.channel_pressure_events += 1,
            Event::PolyphonicKeyPressure(_) => event_metrics.polyphonic_pressure_events += 1,
            Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_) => {
                event_metrics.sysex_events += 1;
            }
            Event::Text(text) => {
                event_metrics.text_events += 1;
                match text.kind {
                    TextEventKind::Lyric => event_metrics.lyric_events += 1,
                    TextEventKind::Marker => event_metrics.marker_events += 1,
                    TextEventKind::CuePoint => event_metrics.cue_point_events += 1,
                    TextEventKind::TrackName => event_metrics.track_name_events += 1,
                    TextEventKind::InstrumentName => event_metrics.instrument_name_events += 1,
                    _ => {}
                }
            }
            Event::Tempo(tempo) => {
                event_metrics.tempo_events += 1;
                tempo_events.push(TempoEventPoint {
                    tick: absolute_tick,
                    track: track_index,
                    event_index,
                    micros_per_quarter: tempo.tempo.max(1),
                });
            }
            Event::TimeSignature(_) => event_metrics.time_signature_events += 1,
            Event::KeySignature(_) => event_metrics.key_signature_events += 1,
            _ => {}
        }

        event_index = event_index.saturating_add(1);
    }

    flush_tick_transition(
        &mut tick_transitions,
        &mut current_transition_tick,
        &mut current_transition_net,
        &mut current_transition_peak,
        &mut current_transition_running,
        track_index,
    );

    for (_, mut starts) in open_notes {
        while let Some(start_tick) = starts.pop_front() {
            *unresolved_starts.entry(start_tick).or_insert(0) += 1;
        }
    }

    Ok(TrackAnalysisPartial {
        track_index,
        total_event_count,
        total_notes,
        max_tick,
        event_metrics,
        velocity_note_on_counts,
        key_note_counts,
        pitch_class_note_counts,
        channel_note_counts,
        track_channel_note_counts,
        tempo_events,
        onset_counts: onset_counts.into_iter().collect(),
        block_starts: block_starts
            .into_iter()
            .map(|((tick, key), count)| (tick, key, count))
            .collect(),
        tick_transitions,
        intervals: interval_counts
            .into_iter()
            .map(|((start_tick, end_tick), count)| NoteIntervalAggregate {
                start_tick,
                end_tick,
                count,
            })
            .collect(),
        unresolved_starts: unresolved_starts.into_iter().collect(),
    })
}

fn reduce_extracted_analysis(
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
            || steps_completed % progress_stride == 0
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
        if steps_completed == total_steps || steps_completed % progress_stride == 0 {
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
            if *steps_completed == total_steps || *steps_completed % progress_stride == 0 {
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
        if *steps_completed == total_steps || *steps_completed % progress_stride == 0 {
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

fn close_note_interval(
    open_notes: &mut FxHashMap<(u8, u8), VecDeque<u64>>,
    interval_counts: &mut FxHashMap<(u64, u64), u64>,
    key: u8,
    channel: u8,
    end_tick: u64,
) -> bool {
    let Some(queue) = open_notes.get_mut(&(key, channel)) else {
        return false;
    };
    let Some(start_tick) = queue.pop_front() else {
        return false;
    };
    if queue.is_empty() {
        open_notes.remove(&(key, channel));
    }
    *interval_counts.entry((start_tick, end_tick)).or_insert(0) += 1;
    true
}

fn record_tick_transition(
    tick_transitions: &mut Vec<TickTransitionPoint>,
    current_transition_tick: &mut Option<u64>,
    current_transition_net: &mut i64,
    current_transition_peak: &mut i64,
    current_transition_running: &mut i64,
    tick: u64,
    delta: i64,
    track: u32,
) {
    if current_transition_tick
        .map(|current| current != tick)
        .unwrap_or(false)
    {
        flush_tick_transition(
            tick_transitions,
            current_transition_tick,
            current_transition_net,
            current_transition_peak,
            current_transition_running,
            track,
        );
    }

    if current_transition_tick.is_none() {
        *current_transition_tick = Some(tick);
    }

    *current_transition_running += delta;
    *current_transition_peak = (*current_transition_peak).max(*current_transition_running);
    *current_transition_net += delta;
}

fn flush_tick_transition(
    tick_transitions: &mut Vec<TickTransitionPoint>,
    current_transition_tick: &mut Option<u64>,
    current_transition_net: &mut i64,
    current_transition_peak: &mut i64,
    current_transition_running: &mut i64,
    track: u32,
) {
    let Some(tick) = current_transition_tick.take() else {
        return;
    };
    if *current_transition_net != 0 || *current_transition_peak != 0 {
        tick_transitions.push(TickTransitionPoint {
            tick,
            track,
            net_delta: *current_transition_net,
            prefix_peak_delta: *current_transition_peak,
        });
    }
    *current_transition_net = 0;
    *current_transition_peak = 0;
    *current_transition_running = 0;
}

fn accumulate_event_metrics(
    target: &mut MidiAnalysisEventMetrics,
    source: &MidiAnalysisEventMetrics,
) {
    target.note_on_events += source.note_on_events;
    target.note_off_events += source.note_off_events;
    target.zero_velocity_note_on_events += source.zero_velocity_note_on_events;
    target.program_change_events += source.program_change_events;
    target.control_change_events += source.control_change_events;
    target.pitch_bend_events += source.pitch_bend_events;
    target.channel_pressure_events += source.channel_pressure_events;
    target.polyphonic_pressure_events += source.polyphonic_pressure_events;
    target.sysex_events += source.sysex_events;
    target.text_events += source.text_events;
    target.lyric_events += source.lyric_events;
    target.marker_events += source.marker_events;
    target.cue_point_events += source.cue_point_events;
    target.track_name_events += source.track_name_events;
    target.instrument_name_events += source.instrument_name_events;
    target.tempo_events += source.tempo_events;
    target.time_signature_events += source.time_signature_events;
    target.key_signature_events += source.key_signature_events;
}

pub(crate) fn build_buckets_from_sparse_data(
    bucket_starts: &[CachedBucketStart],
    bucket_active_deltas: &[CachedBucketDelta],
    bucket_count: usize,
    midi_length: f64,
) -> Vec<MidiAnalysisBucket> {
    let bucket_count = bucket_count.clamp(1, 8192);
    if midi_length <= 0.0 {
        let note_starts = bucket_starts.iter().map(|start| start.count).sum::<u64>();
        let mut active = 0_i64;
        for delta in bucket_active_deltas {
            active += delta.delta;
        }
        return vec![MidiAnalysisBucket {
            time_seconds: 0.0,
            note_starts,
            active_notes: active.max(0) as u64,
        }];
    }

    let bucket_width = midi_length / bucket_count as f64;
    let mut note_starts = vec![0_u64; bucket_count];
    let mut active_deltas = vec![0_i64; bucket_count + 1];

    for start in bucket_starts {
        let start_bucket = bucket_index(start.time_seconds, bucket_width, bucket_count);
        note_starts[start_bucket] += start.count;
    }
    for delta in bucket_active_deltas {
        let target = if delta.delta >= 0 {
            bucket_index(delta.time_seconds, bucket_width, bucket_count)
        } else {
            end_bucket_index(delta.time_seconds, bucket_width, bucket_count)
        };
        active_deltas[target] += delta.delta;
    }

    let mut active = 0_i64;
    let mut buckets = Vec::with_capacity(bucket_count);
    for index in 0..bucket_count {
        active += active_deltas[index];
        buckets.push(MidiAnalysisBucket {
            time_seconds: index as f64 * bucket_width,
            note_starts: note_starts[index],
            active_notes: active.max(0) as u64,
        });
    }
    buckets
}

pub fn build_buckets_from_display_cache(
    cache: &DisplayMidiCache,
    bucket_count: usize,
    midi_length: f64,
) -> Vec<MidiAnalysisBucket> {
    if midi_length <= 0.0 {
        let note_count = cache.note_count();
        return vec![MidiAnalysisBucket {
            time_seconds: 0.0,
            note_starts: note_count,
            active_notes: note_count,
        }];
    }

    let bucket_width = midi_length / bucket_count as f64;
    let mut note_starts = vec![0_u64; bucket_count];
    let mut active_deltas = vec![0_i64; bucket_count + 1];

    for column in cache.columns() {
        for block in column.iter() {
            let start = block.start_seconds;
            let start_bucket = bucket_index(start, bucket_width, bucket_count);
            note_starts[start_bucket] += block.notes.len() as u64;
            active_deltas[start_bucket] += block.notes.len() as i64;
            for note in block.notes.iter() {
                let end = start + note.len_seconds as f64;
                let end_bucket = end_bucket_index(end, bucket_width, bucket_count);
                active_deltas[end_bucket] -= 1;
            }
        }
    }

    let mut active = 0_i64;
    let mut buckets = Vec::with_capacity(bucket_count);
    for index in 0..bucket_count {
        active += active_deltas[index];
        buckets.push(MidiAnalysisBucket {
            time_seconds: index as f64 * bucket_width,
            note_starts: note_starts[index],
            active_notes: active.max(0) as u64,
        });
    }
    buckets
}

fn bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).floor() as usize).min(bucket_count.saturating_sub(1))
}

fn end_bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).ceil() as usize).min(bucket_count)
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
