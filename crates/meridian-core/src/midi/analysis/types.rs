use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::midi::{MIDIAnalysisSummary, display_cache::DisplayMidiCache};

#[derive(Debug, Clone)]
pub(crate) struct CachedBucketStart {
    pub(crate) time_seconds: f64,
    pub(crate) count: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedBucketDelta {
    pub(crate) time_seconds: f64,
    pub(crate) delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MidiAnalysisBucket {
    pub time_seconds: f64,
    pub note_starts: u64,
    pub active_notes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[serde(rename_all = "snake_case")]
pub enum MidiAnalysisKind {
    File,
    Summary,
    Events,
    Notes,
    Tempo,
    Buckets,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MidiAnalysisTempoMetrics {
    pub initial_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub avg_bpm_weighted_by_time: f64,
}

#[derive(Debug, Clone)]
pub struct CachedMidiAnalysis {
    midi_length: f64,
    total_notes: u64,
    total_event_count: u64,
    actual_track_count: usize,
    key_note_counts: Vec<u64>,
    summary: MIDIAnalysisSummary,
    events: MidiAnalysisEventMetrics,
    notes: MidiAnalysisNoteMetrics,
    tempo: MidiAnalysisTempoMetrics,
    bucket_starts: Box<[CachedBucketStart]>,
    bucket_active_deltas: Box<[CachedBucketDelta]>,
}

impl CachedMidiAnalysis {
    pub(crate) fn from_parts(
        midi_length: f64,
        total_notes: u64,
        total_event_count: u64,
        actual_track_count: usize,
        key_note_counts: Vec<u64>,
        summary: MIDIAnalysisSummary,
        events: MidiAnalysisEventMetrics,
        notes: MidiAnalysisNoteMetrics,
        tempo: MidiAnalysisTempoMetrics,
        bucket_starts: Vec<CachedBucketStart>,
        bucket_active_deltas: Vec<CachedBucketDelta>,
    ) -> Self {
        Self {
            midi_length,
            total_notes,
            total_event_count,
            actual_track_count,
            key_note_counts,
            summary,
            events,
            notes,
            tempo,
            bucket_starts: bucket_starts.into_boxed_slice(),
            bucket_active_deltas: bucket_active_deltas.into_boxed_slice(),
        }
    }

    pub fn midi_length(&self) -> f64 {
        self.midi_length
    }

    pub fn total_notes(&self) -> u64 {
        self.total_notes
    }

    pub fn total_event_count(&self) -> u64 {
        self.total_event_count
    }

    pub fn actual_track_count(&self) -> usize {
        self.actual_track_count
    }

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
        super::build_buckets_from_display_cache(cache, bucket_count, midi_length)
    }

    pub(crate) fn build_buckets_from_note_spans(
        &self,
        bucket_count: usize,
    ) -> Vec<MidiAnalysisBucket> {
        super::build_buckets_from_sparse_data(
            &self.bucket_starts,
            &self.bucket_active_deltas,
            bucket_count,
            self.midi_length,
        )
    }

    pub fn note_starts_between(&self, start_seconds: f64, end_seconds: f64) -> u64 {
        let start = start_seconds.min(end_seconds);
        let end = start_seconds.max(end_seconds);
        if end <= start {
            return 0;
        }
        self.note_starts_before(end) - self.note_starts_before(start)
    }

    pub fn active_notes_at(&self, time_seconds: f64) -> u64 {
        let index = self
            .bucket_active_deltas
            .partition_point(|delta| delta.time_seconds <= time_seconds);
        let active = self.bucket_active_deltas[..index]
            .iter()
            .map(|delta| delta.delta)
            .sum::<i64>();
        active.max(0) as u64
    }

    fn note_starts_before(&self, time_seconds: f64) -> u64 {
        let index = self
            .bucket_starts
            .partition_point(|bucket| bucket.time_seconds < time_seconds);
        self.bucket_starts[..index]
            .iter()
            .map(|bucket| bucket.count)
            .sum()
    }
}

impl Default for MidiAnalysisFileMetrics {
    fn default() -> Self {
        Self {
            source_bytes: 0,
            gzip_bytes: 0,
            gzip_ratio: 0.0,
            format: 0,
            declared_track_count: 0,
            actual_track_count: 0,
            ticks_per_quarter: None,
            total_event_count: 0,
        }
    }
}

impl Default for MidiAnalysisNoteMetrics {
    fn default() -> Self {
        Self {
            pitch_class_note_counts: Vec::new(),
            velocity_note_on_counts: Vec::new(),
            track_note_counts: Vec::new(),
            channel_note_counts: Vec::new(),
            track_channel_note_counts: Vec::new(),
            note_start_histogram: Vec::new(),
            total_note_duration_seconds: 0.0,
            avg_note_length_seconds: 0.0,
            min_note_length_seconds: 0.0,
            max_note_length_seconds: 0.0,
            max_simultaneous_notes: 0,
            avg_simultaneous_notes: 0.0,
            notes_per_second_peak: 0.0,
            notes_per_second_avg: 0.0,
            unique_onset_count: 0,
            avg_notes_per_onset: 0.0,
        }
    }
}

impl Default for MidiAnalysisTempoMetrics {
    fn default() -> Self {
        Self {
            initial_bpm: 0.0,
            min_bpm: 0.0,
            max_bpm: 0.0,
            avg_bpm_weighted_by_time: 0.0,
        }
    }
}
