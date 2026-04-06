mod accumulator;
mod file_metrics;
mod types;

use std::{collections::VecDeque, path::PathBuf};

use midi_toolkit::events::{Event, MIDIEventEnum};
use rustc_hash::FxHashMap;

use crate::midi::{TrackAndChannel, display_cache::DisplayMidiCache, parsed::ParsedMidiFile};

pub(crate) use accumulator::MidiAnalysisAccumulator;
use accumulator::{AnalysisKeyState, is_zero_velocity_note_off};
pub use file_metrics::{analyze_file_metrics, gzip_size_for_path};
pub use types::{
    CachedMidiAnalysis, MidiAnalysisBucket, MidiAnalysisData, MidiAnalysisEventMetrics,
    MidiAnalysisFileMetrics, MidiAnalysisKind, MidiAnalysisNoteMetrics, MidiAnalysisTempoMetrics,
    MidiFileInspection,
};

pub fn build_cached_midi_analysis_with_progress(
    parsed: &ParsedMidiFile,
    mut progress: impl FnMut(f32),
) -> Result<CachedMidiAnalysis, crate::error::MeridianError> {
    use midi_toolkit::{
        prelude::ResultIterExt,
        sequence::event::{Delta, Track},
    };

    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(crate::error::MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let merged = midi.iter_all_track_events_merged().unwrap_items();
    let track_count = midi.track_count().max(1);
    let mut keys: Vec<AnalysisKeyState> = (0..256).map(|_| AnalysisKeyState::new()).collect();
    let mut time_seconds = 0.0;
    let mut micros_per_quarter = 500_000_u32;
    let mut total_notes = 0_u64;
    let mut analysis = MidiAnalysisAccumulator::new(track_count);
    let total_events = parsed.total_event_count()?.max(1);
    let progress_stride = (total_events / 200).max(1);
    let mut processed_events = 0_u64;
    progress(0.0);

    type ToolkitEvent = Delta<u64, Track<Event>>;
    for event in merged {
        let event: ToolkitEvent = event;
        processed_events += 1;
        if event.delta > 0 {
            analysis.flush_pending_onset();
            for (key_index, key) in keys.iter_mut().enumerate() {
                key.flush_block(key_index, &mut analysis);
            }
            time_seconds +=
                event.delta as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq as f64;
        }
        analysis.observe_time_advance(time_seconds);

        let track = event.track;
        analysis.observe_event(event.as_event(), time_seconds);
        match event.as_event() {
            Event::Tempo(tempo) => {
                micros_per_quarter = tempo.tempo;
            }
            Event::NoteOn(note_on) => {
                analysis.observe_note_on_velocity(note_on.velocity);
                let key_index = note_on.key as usize;
                if key_index < 256 {
                    let track_chan = TrackAndChannel::new(track, note_on.channel);
                    if is_zero_velocity_note_off(note_on.velocity) {
                        keys[key_index].end_note(
                            key_index,
                            track_chan,
                            time_seconds,
                            &mut analysis,
                        );
                    } else {
                        keys[key_index].add_note(track_chan, time_seconds);
                        analysis.observe_note_start(time_seconds, key_index, track_chan);
                        total_notes += 1;
                    }
                }
            }
            Event::NoteOff(note_off) => {
                analysis.observe_note_off();
                let key_index = note_off.key as usize;
                if key_index < 256 {
                    keys[key_index].end_note(
                        key_index,
                        TrackAndChannel::new(track, note_off.channel),
                        time_seconds,
                        &mut analysis,
                    );
                }
            }
            _ => {}
        }

        if processed_events == 1
            || processed_events >= total_events
            || processed_events % progress_stride == 0
        {
            progress((processed_events as f32 / total_events as f32).clamp(0.0, 1.0));
        }
    }

    analysis.flush_pending_onset();
    for (key_index, key) in keys.iter_mut().enumerate() {
        key.flush_block(key_index, &mut analysis);
        key.end_all(time_seconds, &mut analysis);
    }
    progress(1.0);

    Ok(analysis.finalize(time_seconds, total_notes, track_count))
}

pub fn build_buckets_from_parsed_with_progress(
    parsed: &ParsedMidiFile,
    bucket_count: usize,
    midi_length: f64,
    mut progress: impl FnMut(f32),
) -> Result<Vec<MidiAnalysisBucket>, crate::error::MeridianError> {
    use midi_toolkit::{
        prelude::ResultIterExt,
        sequence::event::{Delta, Track},
    };

    let bucket_count = bucket_count.clamp(1, 8192);
    if midi_length <= 0.0 {
        return Ok(vec![MidiAnalysisBucket {
            time_seconds: 0.0,
            note_starts: 0,
            active_notes: 0,
        }]);
    }

    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(crate::error::MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let merged = midi.iter_all_track_events_merged().unwrap_items();
    let total_events = parsed.total_event_count()?.max(1);
    let progress_stride = (total_events / 200).max(1);
    let bucket_width = midi_length / bucket_count as f64;
    let mut note_starts = vec![0_u64; bucket_count];
    let mut active_deltas = vec![0_i64; bucket_count + 1];
    let mut open_notes: FxHashMap<(u8, TrackAndChannel), VecDeque<f64>> = FxHashMap::default();
    let mut micros_per_quarter = 500_000_u32;
    let mut time_seconds = 0.0;
    let mut processed_events = 0_u64;
    progress(0.0);

    type ToolkitEvent = Delta<u64, Track<Event>>;
    for event in merged {
        let event: ToolkitEvent = event;
        processed_events += 1;
        if event.delta > 0 {
            time_seconds +=
                event.delta as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq as f64;
        }

        let track = event.track;
        match event.as_event() {
            Event::Tempo(tempo) => {
                micros_per_quarter = tempo.tempo;
            }
            Event::NoteOn(note_on) => {
                let key = (note_on.key, TrackAndChannel::new(track, note_on.channel));
                if is_zero_velocity_note_off(note_on.velocity) {
                    if let Some(queue) = open_notes.get_mut(&key) {
                        let removed = queue.pop_front().is_some();
                        if queue.is_empty() {
                            open_notes.remove(&key);
                        }
                        if removed {
                            let end_bucket =
                                end_bucket_index(time_seconds, bucket_width, bucket_count);
                            active_deltas[end_bucket] -= 1;
                        }
                    }
                } else {
                    let start_bucket = bucket_index(time_seconds, bucket_width, bucket_count);
                    note_starts[start_bucket] += 1;
                    active_deltas[start_bucket] += 1;
                    open_notes.entry(key).or_default().push_back(time_seconds);
                }
            }
            Event::NoteOff(note_off) => {
                let key = (note_off.key, TrackAndChannel::new(track, note_off.channel));
                if let Some(queue) = open_notes.get_mut(&key) {
                    let _ = queue.pop_front();
                    if queue.is_empty() {
                        open_notes.remove(&key);
                    }
                }
                let end_bucket = end_bucket_index(time_seconds, bucket_width, bucket_count);
                active_deltas[end_bucket] -= 1;
            }
            _ => {}
        }

        if processed_events == 1
            || processed_events >= total_events
            || processed_events % progress_stride == 0
        {
            progress((processed_events as f32 / total_events as f32).clamp(0.0, 1.0));
        }
    }

    let end_bucket = end_bucket_index(midi_length, bucket_width, bucket_count);
    for (_, mut queue) in open_notes.drain() {
        while queue.pop_front().is_some() {
            active_deltas[end_bucket] -= 1;
        }
    }
    progress(1.0);

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
    Ok(buckets)
}

pub fn analyze_cached_midi(
    parsed: &ParsedMidiFile,
    cached: &CachedMidiAnalysis,
    buckets: Vec<MidiAnalysisBucket>,
) -> MidiAnalysisData {
    MidiAnalysisData {
        midi_length: cached.midi_length(),
        total_notes: cached.total_notes(),
        key_note_counts: cached.key_note_counts().to_vec(),
        summary: cached.summary(),
        buckets,
        file: analyze_file_metrics(parsed, cached.actual_track_count()),
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
    progress: impl FnMut(f32),
) -> Result<MidiAnalysisData, crate::error::MeridianError> {
    let buckets = build_buckets_from_parsed_with_progress(
        parsed,
        bucket_count,
        cached.midi_length(),
        progress,
    )?;
    Ok(analyze_cached_midi(parsed, cached, buckets))
}

pub fn inspect_midi_files(paths: &[PathBuf]) -> Vec<MidiFileInspection> {
    paths.iter().cloned().map(inspect_midi_file).collect()
}

pub fn inspect_midi_file(path: PathBuf) -> MidiFileInspection {
    let inspection = || -> Result<MidiFileInspection, crate::error::MeridianError> {
        let parsed = ParsedMidiFile::load_from_file(path.clone())?;
        let cached = build_cached_midi_analysis_with_progress(&parsed, |_| {})?;
        let file = analyze_file_metrics(&parsed, cached.actual_track_count());
        Ok(MidiFileInspection {
            path: path.clone(),
            file_bytes: file.source_bytes,
            midi_length: cached.midi_length(),
            total_notes: cached.total_notes(),
            total_event_count: file.total_event_count,
            declared_track_count: file.declared_track_count,
            actual_track_count: file.actual_track_count,
            ticks_per_quarter: file.ticks_per_quarter,
            tempo_event_count: cached.events().tempo_events,
            time_signature_event_count: cached.events().time_signature_events,
            key_signature_event_count: cached.events().key_signature_events,
            track_name_event_count: cached.events().track_name_events,
            initial_bpm: cached.tempo().initial_bpm,
            error: None,
        })
    };

    match inspection() {
        Ok(inspection) => inspection,
        Err(error) => MidiFileInspection {
            path,
            file_bytes: 0,
            midi_length: 0.0,
            total_notes: 0,
            total_event_count: 0,
            declared_track_count: 0,
            actual_track_count: 0,
            ticks_per_quarter: None,
            tempo_event_count: 0,
            time_signature_event_count: 0,
            key_signature_event_count: 0,
            track_name_event_count: 0,
            initial_bpm: 0.0,
            error: Some(error.to_string()),
        },
    }
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
        analysis.summary = crate::midi::MIDIAnalysisSummary {
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
