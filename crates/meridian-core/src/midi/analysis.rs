use std::{
    fs::File,
    io::{Read, Write},
};

use flate2::{Compression, write::GzEncoder};
use midi_toolkit::events::{Event, TextEventKind};
use serde::{Deserialize, Serialize};

use crate::midi::{
    MIDIAnalysisSummary, TrackAndChannel, display_cache::DisplayMidiCache, parsed::ParsedMidiFile,
};

const NOTE_START_HISTOGRAM_BINS: usize = 257;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAnalysisBucket {
    pub time_seconds: f64,
    pub note_starts: u64,
    pub active_notes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAnalysisData {
    pub midi_length: f64,
    pub total_notes: u64,
    pub key_note_counts: Vec<u64>,
    pub summary: MIDIAnalysisSummary,
    pub buckets: Vec<MidiAnalysisBucket>,
    pub file: MidiAnalysisFileMetrics,
    pub events: MidiAnalysisEventMetrics,
    pub notes: MidiAnalysisNoteMetrics,
    pub tempo: MidiAnalysisTempoMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAnalysisFileMetrics {
    pub source_bytes: u64,
    pub gzip_bytes: u64,
    pub gzip_ratio: f64,
    pub format: u16,
    pub declared_track_count: u16,
    pub actual_track_count: usize,
    pub ticks_per_quarter: Option<u16>,
    pub total_event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MidiAnalysisEventMetrics {
    pub note_on_events: u64,
    pub note_off_events: u64,
    pub zero_velocity_note_on_events: u64,
    pub program_change_events: u64,
    pub control_change_events: u64,
    pub pitch_bend_events: u64,
    pub channel_pressure_events: u64,
    pub polyphonic_pressure_events: u64,
    pub sysex_events: u64,
    pub text_events: u64,
    pub lyric_events: u64,
    pub marker_events: u64,
    pub cue_point_events: u64,
    pub track_name_events: u64,
    pub instrument_name_events: u64,
    pub tempo_events: u64,
    pub time_signature_events: u64,
    pub key_signature_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAnalysisNoteMetrics {
    pub pitch_class_note_counts: Vec<u64>,
    pub velocity_note_on_counts: Vec<u64>,
    pub track_note_counts: Vec<u64>,
    pub channel_note_counts: Vec<u64>,
    pub track_channel_note_counts: Vec<u64>,
    pub note_start_histogram: Vec<u64>,
    pub total_note_duration_seconds: f64,
    pub avg_note_length_seconds: f64,
    pub min_note_length_seconds: f64,
    pub max_note_length_seconds: f64,
    pub max_simultaneous_notes: u64,
    pub avg_simultaneous_notes: f64,
    pub notes_per_second_peak: f64,
    pub notes_per_second_avg: f64,
    pub unique_onset_count: u64,
    pub avg_notes_per_onset: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiAnalysisTempoMetrics {
    pub initial_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub avg_bpm_weighted_by_time: f64,
}

#[derive(Debug, Clone)]
pub struct CachedMidiAnalysis {
    key_note_counts: Vec<u64>,
    summary: MIDIAnalysisSummary,
    events: MidiAnalysisEventMetrics,
    notes: MidiAnalysisNoteMetrics,
    tempo: MidiAnalysisTempoMetrics,
}

impl CachedMidiAnalysis {
    pub fn key_note_counts(&self) -> &[u64] {
        &self.key_note_counts
    }

    pub fn summary(&self) -> MIDIAnalysisSummary {
        self.summary
    }

    pub fn events(&self) -> &MidiAnalysisEventMetrics {
        &self.events
    }

    pub fn notes(&self) -> &MidiAnalysisNoteMetrics {
        &self.notes
    }

    pub fn tempo(&self) -> &MidiAnalysisTempoMetrics {
        &self.tempo
    }

    pub fn build_buckets(
        &self,
        cache: &DisplayMidiCache,
        bucket_count: usize,
        midi_length: f64,
    ) -> Vec<MidiAnalysisBucket> {
        build_buckets_from_display_cache(cache, bucket_count, midi_length)
    }
}

pub(crate) struct MidiAnalysisAccumulator {
    key_note_counts: Vec<u64>,
    pitch_class_note_counts: Vec<u64>,
    velocity_note_on_counts: Vec<u64>,
    track_note_counts: Vec<u64>,
    channel_note_counts: Vec<u64>,
    track_channel_note_counts: Vec<u64>,
    note_start_histogram: Vec<u64>,
    total_note_duration_seconds: f64,
    min_note_length_seconds: f64,
    max_note_length_seconds: f64,
    unique_onset_count: u64,
    total_blocks: u64,
    keys_with_notes: usize,
    max_blocks_per_key: usize,
    max_notes_in_block: usize,
    densest_key: usize,
    densest_key_notes: u64,
    block_counts_per_key: Vec<usize>,
    pending_onset_time: Option<f64>,
    pending_onset_notes: u64,
    events: MidiAnalysisEventMetrics,
    initial_bpm: f64,
    current_bpm: f64,
    min_bpm: f64,
    max_bpm: f64,
    weighted_bpm_sum: f64,
    last_tempo_seconds: f64,
    last_observed_time_seconds: f64,
    active_notes: i64,
    polyphony_area: f64,
    max_simultaneous_notes: u64,
    rolling_nps_peak: RollingNpsPeak,
}

impl MidiAnalysisAccumulator {
    pub fn new(track_count: usize) -> Self {
        Self {
            key_note_counts: vec![0; 256],
            pitch_class_note_counts: vec![0; 12],
            velocity_note_on_counts: vec![0; 128],
            track_note_counts: vec![0; track_count],
            channel_note_counts: vec![0; 16],
            track_channel_note_counts: vec![0; track_count * 16],
            note_start_histogram: vec![0; NOTE_START_HISTOGRAM_BINS],
            total_note_duration_seconds: 0.0,
            min_note_length_seconds: f64::INFINITY,
            max_note_length_seconds: 0.0,
            unique_onset_count: 0,
            total_blocks: 0,
            keys_with_notes: 0,
            max_blocks_per_key: 0,
            max_notes_in_block: 0,
            densest_key: 0,
            densest_key_notes: 0,
            block_counts_per_key: vec![0; 256],
            pending_onset_time: None,
            pending_onset_notes: 0,
            events: MidiAnalysisEventMetrics::default(),
            initial_bpm: 120.0,
            current_bpm: 120.0,
            min_bpm: 120.0,
            max_bpm: 120.0,
            weighted_bpm_sum: 0.0,
            last_tempo_seconds: 0.0,
            last_observed_time_seconds: 0.0,
            active_notes: 0,
            polyphony_area: 0.0,
            max_simultaneous_notes: 0,
            rolling_nps_peak: RollingNpsPeak::new(),
        }
    }

    pub fn observe_time_advance(&mut self, next_time_seconds: f64) {
        let span = (next_time_seconds - self.last_observed_time_seconds).max(0.0);
        self.polyphony_area += self.active_notes.max(0) as f64 * span;
        self.last_observed_time_seconds = next_time_seconds;
    }

    pub fn flush_pending_onset(&mut self) {
        if self.pending_onset_notes == 0 {
            self.pending_onset_time = None;
            return;
        }

        let time = self.pending_onset_time.unwrap_or(0.0);
        self.unique_onset_count += 1;
        let onset_size = (self.pending_onset_notes as usize).min(NOTE_START_HISTOGRAM_BINS - 1);
        self.note_start_histogram[onset_size] += 1;
        self.rolling_nps_peak.push(time, self.pending_onset_notes);
        self.pending_onset_time = None;
        self.pending_onset_notes = 0;
    }

    pub fn observe_note_on_velocity(&mut self, velocity: u8) {
        self.events.note_on_events += 1;
        if let Some(count) = self.velocity_note_on_counts.get_mut(velocity as usize) {
            *count += 1;
        }
        if velocity == 0 {
            self.events.zero_velocity_note_on_events += 1;
        }
    }

    pub fn observe_note_off(&mut self) {
        self.events.note_off_events += 1;
    }

    pub fn observe_event(&mut self, event: &Event, time_seconds: f64) {
        match event {
            Event::ProgramChange(_) => self.events.program_change_events += 1,
            Event::ControlChange(_) => self.events.control_change_events += 1,
            Event::PitchWheelChange(_) => self.events.pitch_bend_events += 1,
            Event::ChannelPressure(_) => self.events.channel_pressure_events += 1,
            Event::PolyphonicKeyPressure(_) => self.events.polyphonic_pressure_events += 1,
            Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_) => {
                self.events.sysex_events += 1;
            }
            Event::Text(text) => {
                self.events.text_events += 1;
                match text.kind {
                    TextEventKind::Lyric => self.events.lyric_events += 1,
                    TextEventKind::Marker => self.events.marker_events += 1,
                    TextEventKind::CuePoint => self.events.cue_point_events += 1,
                    TextEventKind::TrackName => self.events.track_name_events += 1,
                    TextEventKind::InstrumentName => self.events.instrument_name_events += 1,
                    _ => {}
                }
            }
            Event::Tempo(tempo) => {
                self.events.tempo_events += 1;
                let span = (time_seconds - self.last_tempo_seconds).max(0.0);
                self.weighted_bpm_sum += self.current_bpm * span;
                self.last_tempo_seconds = time_seconds;
                self.current_bpm = 60_000_000.0 / tempo.tempo.max(1) as f64;
                if time_seconds == 0.0 {
                    self.initial_bpm = self.current_bpm;
                }
                self.min_bpm = self.min_bpm.min(self.current_bpm);
                self.max_bpm = self.max_bpm.max(self.current_bpm);
            }
            Event::TimeSignature(_) => self.events.time_signature_events += 1,
            Event::KeySignature(_) => self.events.key_signature_events += 1,
            _ => {}
        }
    }

    pub fn observe_note_start(
        &mut self,
        time_seconds: f64,
        key: usize,
        track_chan: TrackAndChannel,
    ) {
        if self.pending_onset_time != Some(time_seconds) {
            self.flush_pending_onset();
            self.pending_onset_time = Some(time_seconds);
        }
        self.pending_onset_notes += 1;

        self.key_note_counts[key] += 1;
        self.pitch_class_note_counts[key % 12] += 1;
        self.track_note_counts[track_chan.track()] += 1;
        self.channel_note_counts[track_chan.channel()] += 1;
        self.track_channel_note_counts[track_chan.as_usize()] += 1;
        self.active_notes += 1;
        self.max_simultaneous_notes = self
            .max_simultaneous_notes
            .max(self.active_notes.max(0) as u64);
    }

    pub fn observe_note_end(&mut self, duration_seconds: f64) {
        self.total_note_duration_seconds += duration_seconds;
        self.min_note_length_seconds = self.min_note_length_seconds.min(duration_seconds);
        self.max_note_length_seconds = self.max_note_length_seconds.max(duration_seconds);
        self.active_notes = self.active_notes.saturating_sub(1);
    }

    pub fn observe_block(&mut self, key: usize, note_count: usize) {
        self.total_blocks += 1;
        if self.block_counts_per_key[key] == 0 {
            self.keys_with_notes += 1;
        }
        self.block_counts_per_key[key] += 1;
        self.max_blocks_per_key = self.max_blocks_per_key.max(self.block_counts_per_key[key]);
        self.max_notes_in_block = self.max_notes_in_block.max(note_count);
        let key_notes = self.key_note_counts[key];
        if key_notes > self.densest_key_notes {
            self.densest_key = key;
            self.densest_key_notes = key_notes;
        }
    }

    pub fn finalize(mut self, midi_length: f64, total_notes: u64) -> CachedMidiAnalysis {
        self.flush_pending_onset();
        let trailing_span = (midi_length - self.last_tempo_seconds).max(0.0);
        self.weighted_bpm_sum += self.current_bpm * trailing_span;
        let trailing_polyphony_span = (midi_length - self.last_observed_time_seconds).max(0.0);
        self.polyphony_area += self.active_notes.max(0) as f64 * trailing_polyphony_span;

        let avg_simultaneous_notes = if midi_length > 0.0 {
            self.polyphony_area / midi_length
        } else {
            0.0
        };
        let notes_per_second_avg = if midi_length > 0.0 {
            total_notes as f64 / midi_length
        } else {
            total_notes as f64
        };
        let avg_notes_per_onset = if self.unique_onset_count > 0 {
            total_notes as f64 / self.unique_onset_count as f64
        } else {
            0.0
        };
        let avg_note_length_seconds = if total_notes > 0 {
            self.total_note_duration_seconds / total_notes as f64
        } else {
            0.0
        };

        CachedMidiAnalysis {
            key_note_counts: self.key_note_counts.clone(),
            summary: MIDIAnalysisSummary {
                total_blocks: self.total_blocks,
                keys_with_notes: self.keys_with_notes,
                max_blocks_per_key: self.max_blocks_per_key,
                max_notes_in_block: self.max_notes_in_block,
                densest_key: self.densest_key,
                densest_key_notes: self.densest_key_notes,
            },
            events: self.events,
            notes: MidiAnalysisNoteMetrics {
                pitch_class_note_counts: self.pitch_class_note_counts,
                velocity_note_on_counts: self.velocity_note_on_counts,
                track_note_counts: self.track_note_counts,
                channel_note_counts: self.channel_note_counts,
                track_channel_note_counts: self.track_channel_note_counts,
                note_start_histogram: self.note_start_histogram,
                total_note_duration_seconds: self.total_note_duration_seconds,
                avg_note_length_seconds,
                min_note_length_seconds: if self.min_note_length_seconds.is_finite() {
                    self.min_note_length_seconds
                } else {
                    0.0
                },
                max_note_length_seconds: self.max_note_length_seconds,
                max_simultaneous_notes: self.max_simultaneous_notes,
                avg_simultaneous_notes,
                notes_per_second_peak: self.rolling_nps_peak.peak() as f64,
                notes_per_second_avg,
                unique_onset_count: self.unique_onset_count,
                avg_notes_per_onset,
            },
            tempo: MidiAnalysisTempoMetrics {
                initial_bpm: self.initial_bpm,
                min_bpm: self.min_bpm,
                max_bpm: self.max_bpm,
                avg_bpm_weighted_by_time: if midi_length > 0.0 {
                    self.weighted_bpm_sum / midi_length
                } else {
                    self.current_bpm
                },
            },
        }
    }

    pub fn into_note_metrics_only(
        mut self,
        midi_length: f64,
        total_notes: u64,
    ) -> CachedMidiAnalysis {
        self.events = MidiAnalysisEventMetrics::default();
        self.finalize(midi_length, total_notes)
    }
}

pub fn analyze_midi(
    parsed: &ParsedMidiFile,
    cache: &DisplayMidiCache,
    cached: &CachedMidiAnalysis,
    bucket_count: usize,
) -> MidiAnalysisData {
    let bucket_count = bucket_count.clamp(1, 8192);
    let midi_length = cache.length().max(0.0);
    let total_notes = cache.note_count();

    MidiAnalysisData {
        midi_length,
        total_notes,
        key_note_counts: cached.key_note_counts().to_vec(),
        summary: cached.summary(),
        buckets: cached.build_buckets(cache, bucket_count, midi_length),
        file: analyze_file_metrics(parsed, cache.track_count()),
        events: cached.events().clone(),
        notes: cached.notes().clone(),
        tempo: cached.tempo().clone(),
    }
}

pub fn analyze_file_metrics(
    parsed: &ParsedMidiFile,
    actual_track_count: usize,
) -> MidiAnalysisFileMetrics {
    let header = parsed.header();
    let source_bytes = parsed.signature().length_in_bytes;
    let gzip_bytes = parsed.cached_gzip_size().unwrap_or(0);
    MidiAnalysisFileMetrics {
        source_bytes,
        gzip_bytes,
        gzip_ratio: if source_bytes > 0 {
            gzip_bytes as f64 / source_bytes as f64
        } else {
            0.0
        },
        format: header.format,
        declared_track_count: header.declared_track_count,
        actual_track_count,
        ticks_per_quarter: ((header.time_division & 0x8000) == 0).then_some(header.time_division),
        total_event_count: parsed.total_event_count(),
    }
}

fn build_buckets_from_display_cache(
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

fn bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).floor() as usize).min(bucket_count.saturating_sub(1))
}

fn end_bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).ceil() as usize).min(bucket_count)
}

pub fn gzip_size_for_path(path: &std::path::Path) -> std::io::Result<u64> {
    let mut reader = File::open(path)?;
    let mut writer = CountingWriter::default();
    {
        let mut encoder = GzEncoder::new(&mut writer, Compression::fast());
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            encoder.write_all(&buffer[..read])?;
        }
        let _ = encoder.finish()?;
    }
    Ok(writer.bytes_written)
}

#[derive(Default)]
struct CountingWriter {
    bytes_written: u64,
}

impl Write for CountingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes_written += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
