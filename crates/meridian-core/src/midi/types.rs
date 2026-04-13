use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::render::DisplayTimeSpace;

pub const MIDI_KEY_COUNT: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
pub struct MIDIAnalysisSummary {
    pub total_blocks: u64,
    pub keys_with_notes: usize,
    pub max_blocks_per_key: usize,
    pub max_notes_in_block: usize,
    pub densest_key: usize,
    pub densest_key_notes: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIFileStats {
    pub total_notes: Option<u64>,
    pub passed_notes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
pub struct MidiBuildProgress {
    pub fraction_complete: Option<f32>,
    pub completed_events: Option<u64>,
    pub completed_notes: Option<u64>,
    pub total_events: Option<u64>,
}

impl MidiBuildProgress {
    pub fn from_fraction(fraction_complete: f32) -> Self {
        Self {
            fraction_complete: Some(fraction_complete.clamp(0.0, 1.0)),
            completed_events: None,
            completed_notes: None,
            total_events: None,
        }
    }

    pub fn from_counts(
        completed_events: u64,
        completed_notes: u64,
        total_events: Option<u64>,
    ) -> Self {
        let fraction_complete = total_events.map(|total_events| {
            if total_events == 0 {
                1.0
            } else {
                (completed_events as f32 / total_events as f32).clamp(0.0, 1.0)
            }
        });
        Self {
            fraction_complete,
            completed_events: Some(completed_events),
            completed_notes: Some(completed_notes),
            total_events,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MIDIViewRange {
    pub start: f64,
    pub end: f64,
    pub time_space: DisplayTimeSpace,
    pub scale: f64,
}

impl MIDIViewRange {
    pub fn new(start: f64, end: f64, time_space: DisplayTimeSpace, scale: f64) -> Self {
        Self {
            start,
            end,
            time_space,
            scale,
        }
    }

    pub fn length(&self) -> f64 {
        (self.end - self.start) * self.scale
    }
}

impl Default for MIDIViewRange {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            time_space: DisplayTimeSpace::Time,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MIDIFileUniqueSignature {
    pub filepath: std::path::PathBuf,
    pub length_in_bytes: u64,
    pub last_modified: u128,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) struct TrackAndChannel(u32);

impl TrackAndChannel {
    pub(crate) fn new(track: u32, channel: u8) -> Self {
        Self(track * 16 + channel as u32)
    }

    pub(crate) fn track(self) -> usize {
        (self.0 / 16) as usize
    }

    pub(crate) fn channel(self) -> usize {
        (self.0 % 16) as usize
    }

    pub(crate) fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplacedMIDINote {
    pub start: f32,
    pub len: f32,
    pub color: crate::midi::MIDIColorPair,
}
