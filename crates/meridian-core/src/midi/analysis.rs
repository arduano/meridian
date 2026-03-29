use serde::{Deserialize, Serialize};

use crate::{
    midi::{MIDIAnalysisSummary, display_cache::DisplayMidiCache},
    render::DisplayTimeSpace,
};

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
}

pub fn analyze_display_cache(cache: &DisplayMidiCache, bucket_count: usize) -> MidiAnalysisData {
    let bucket_count = bucket_count.clamp(1, 8192);
    let midi_length = cache.length().max(0.0);
    let total_notes = cache.note_count();
    let summary = analyze_summary(cache);
    let key_note_counts = key_note_counts(cache);
    let buckets = analyze_buckets(cache, bucket_count, midi_length);

    MidiAnalysisData {
        midi_length,
        total_notes,
        key_note_counts,
        summary,
        buckets,
    }
}

fn analyze_summary(cache: &DisplayMidiCache) -> MIDIAnalysisSummary {
    let mut total_blocks = 0_u64;
    let mut keys_with_notes = 0_usize;
    let mut max_blocks_per_key = 0_usize;
    let mut max_notes_in_block = 0_usize;
    let mut densest_key = 0_usize;
    let mut densest_key_notes = 0_u64;

    for (key, column) in cache.columns().iter().enumerate() {
        let block_count = column.len();
        let note_count = column
            .iter()
            .map(|block| block.notes.len() as u64)
            .sum::<u64>();

        total_blocks += block_count as u64;
        if note_count > 0 {
            keys_with_notes += 1;
        }
        max_blocks_per_key = max_blocks_per_key.max(block_count);
        if note_count > densest_key_notes {
            densest_key = key;
            densest_key_notes = note_count;
        }
        for block in column.iter() {
            max_notes_in_block = max_notes_in_block.max(block.notes.len());
        }
    }

    MIDIAnalysisSummary {
        total_blocks,
        keys_with_notes,
        max_blocks_per_key,
        max_notes_in_block,
        densest_key,
        densest_key_notes,
    }
}

fn key_note_counts(cache: &DisplayMidiCache) -> Vec<u64> {
    cache
        .columns()
        .iter()
        .map(|column| column.iter().map(|block| block.notes.len() as u64).sum())
        .collect()
}

fn analyze_buckets(
    cache: &DisplayMidiCache,
    bucket_count: usize,
    midi_length: f64,
) -> Vec<MidiAnalysisBucket> {
    if midi_length <= 0.0 {
        return vec![MidiAnalysisBucket {
            time_seconds: 0.0,
            note_starts: cache.note_count(),
            active_notes: cache.note_count(),
        }];
    }

    let bucket_width = midi_length / bucket_count as f64;
    let mut note_starts = vec![0_u64; bucket_count];
    let mut active_deltas = vec![0_i64; bucket_count + 1];

    for column in cache.columns() {
        for block in column.iter() {
            let start_index = bucket_index(
                block.start(DisplayTimeSpace::Time),
                bucket_width,
                bucket_count,
            );
            note_starts[start_index] += block.notes.len() as u64;

            for note in block.notes.iter() {
                let note_start = block.start(DisplayTimeSpace::Time);
                let note_end = note_start + note.len_seconds as f64;
                let start = bucket_index(note_start, bucket_width, bucket_count);
                let end_exclusive = end_bucket_index(note_end, bucket_width, bucket_count);
                active_deltas[start] += 1;
                active_deltas[end_exclusive] -= 1;
            }
        }
    }

    let mut active_notes = 0_i64;
    let mut buckets = Vec::with_capacity(bucket_count);
    for index in 0..bucket_count {
        active_notes += active_deltas[index];
        buckets.push(MidiAnalysisBucket {
            time_seconds: index as f64 * bucket_width,
            note_starts: note_starts[index],
            active_notes: active_notes.max(0) as u64,
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
