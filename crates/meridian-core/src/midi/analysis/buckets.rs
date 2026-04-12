use crate::midi::display_cache::DisplayMidiCache;

use super::types::{CachedBucketDelta, CachedBucketStart, MidiAnalysisBucket};

pub(crate) fn build_buckets_from_sparse_data(
    bucket_starts: &[CachedBucketStart],
    bucket_active_deltas: &[CachedBucketDelta],
    bucket_count: usize,
    midi_length: f64,
) -> Vec<MidiAnalysisBucket> {
    let Some(bucket_count) = normalized_bucket_count(bucket_count) else {
        return Vec::new();
    };
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
    let Some(bucket_count) = normalized_bucket_count(bucket_count) else {
        return Vec::new();
    };
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

pub(crate) fn normalized_bucket_count(bucket_count: usize) -> Option<usize> {
    if bucket_count == 0 {
        None
    } else {
        Some(bucket_count.min(8192))
    }
}

fn bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).floor() as usize).min(bucket_count.saturating_sub(1))
}

fn end_bucket_index(time: f64, width: f64, bucket_count: usize) -> usize {
    ((time / width).ceil() as usize).min(bucket_count)
}

#[cfg(test)]
mod tests {
    use super::{
        build_buckets_from_sparse_data, normalized_bucket_count, CachedBucketDelta,
        CachedBucketStart,
    };

    #[test]
    fn zero_bucket_count_returns_empty_sparse_buckets() {
        let buckets = build_buckets_from_sparse_data(
            &[CachedBucketStart {
                time_seconds: 0.25,
                count: 2,
            }],
            &[CachedBucketDelta {
                time_seconds: 0.5,
                delta: 1,
            }],
            0,
            1.0,
        );

        assert!(buckets.is_empty());
    }

    #[test]
    fn normalized_bucket_count_preserves_zero_as_none() {
        assert_eq!(normalized_bucket_count(0), None);
        assert_eq!(normalized_bucket_count(1), Some(1));
        assert_eq!(normalized_bucket_count(10_000), Some(8192));
    }
}
