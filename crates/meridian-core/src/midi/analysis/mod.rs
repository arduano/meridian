pub struct AnalysisProgressUpdate {
    pub progress: f32,
    pub status: String,
}

mod accumulator;
mod buckets;
mod extract;
mod file_metrics;
mod reduce;
mod types;

use std::sync::Mutex;

use crate::midi::{display_cache::DisplayMidiCache, parsed::ParsedMidiFile, MIDIAnalysisSummary};

pub(crate) use accumulator::MidiAnalysisAccumulator;
pub use buckets::build_buckets_from_display_cache;
pub use file_metrics::{
    analyze_file_metrics, analyze_file_metrics_with_values, gzip_size_for_path,
};
use buckets::normalized_bucket_count;
use extract::extract_analysis;
use reduce::reduce_extracted_analysis;
pub use types::{
    CachedMidiAnalysis, MidiAnalysisBucket, MidiAnalysisData, MidiAnalysisEventMetrics,
    MidiAnalysisFileMetrics, MidiAnalysisKind, MidiAnalysisNoteMetrics, MidiAnalysisTempoMetrics,
};

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
    let buckets = match normalized_bucket_count(bucket_count) {
        Some(bucket_count) => cached.build_buckets_from_note_spans(bucket_count),
        None => Vec::new(),
    };
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
    let buckets = normalized_bucket_count(bucket_count)
        .map(|bucket_count| cached.build_buckets(cache, bucket_count, cached.midi_length()))
        .unwrap_or_default();
    analyze_cached_midi(parsed, cached, buckets)
}

pub fn analyze_parsed_midi_with_progress(
    parsed: &ParsedMidiFile,
    cached: &CachedMidiAnalysis,
    bucket_count: usize,
    mut progress: impl FnMut(f32),
) -> Result<MidiAnalysisData, crate::error::MeridianError> {
    progress(0.0);
    let buckets = normalized_bucket_count(bucket_count)
        .map(|bucket_count| cached.build_buckets_from_note_spans(bucket_count))
        .unwrap_or_default();
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

pub(crate) fn build_buckets_from_sparse_data(
    bucket_starts: &[types::CachedBucketStart],
    bucket_active_deltas: &[types::CachedBucketDelta],
    bucket_count: usize,
    midi_length: f64,
) -> Vec<MidiAnalysisBucket> {
    buckets::build_buckets_from_sparse_data(
        bucket_starts,
        bucket_active_deltas,
        bucket_count,
        midi_length,
    )
}
