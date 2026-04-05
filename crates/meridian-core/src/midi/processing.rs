use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tools::MidiModifierTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct MidiProcessingConfig {
    pub time: TimeProcessingConfig,
    pub notes: NoteProcessingConfig,
    pub events: EventFilterConfig,
    pub piano_only: bool,
    pub zero_velocity_note_on: ZeroVelocityNoteOnMode,
}

impl Default for MidiProcessingConfig {
    fn default() -> Self {
        Self {
            time: TimeProcessingConfig::default(),
            notes: NoteProcessingConfig::default(),
            events: EventFilterConfig::default(),
            piano_only: false,
            zero_velocity_note_on: ZeroVelocityNoteOnMode::NoteOff,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct TimeProcessingConfig {
    pub offset_seconds: f64,
}

impl Default for TimeProcessingConfig {
    fn default() -> Self {
        Self {
            offset_seconds: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct NoteProcessingConfig {
    pub min_key: u8,
    pub max_key: u8,
    pub transpose: i16,
    pub velocity_scale: f32,
}

impl Default for NoteProcessingConfig {
    fn default() -> Self {
        Self {
            min_key: 0,
            max_key: 127,
            transpose: 0,
            velocity_scale: 1.0,
        }
    }
}

impl NoteProcessingConfig {
    pub fn map_key(&self, key: u8) -> Option<u8> {
        if key < self.min_key || key > self.max_key {
            return None;
        }
        let mapped = key as i16 + self.transpose;
        (0..=255).contains(&mapped).then_some(mapped as u8)
    }

    pub fn map_velocity(&self, velocity: u8) -> u8 {
        ((velocity as f32 * self.velocity_scale).round() as i32).clamp(0, 127) as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct EventFilterConfig {
    pub notes: bool,
    pub tempo: bool,
    pub pitch_bend: bool,
    pub channel_controls: bool,
    pub program_changes: bool,
    pub channel_pressure: bool,
    pub polyphonic_pressure: bool,
    pub sysex: bool,
    pub meta_other: bool,
}

impl Default for EventFilterConfig {
    fn default() -> Self {
        Self {
            notes: true,
            tempo: true,
            pitch_bend: true,
            channel_controls: true,
            program_changes: true,
            channel_pressure: true,
            polyphonic_pressure: true,
            sysex: true,
            meta_other: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ZeroVelocityNoteOnMode {
    NoteOff,
    KeepAsNoteOn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct MidiFileProcessingConfig {
    pub time: FileTimeProcessingConfig,
    pub notes: NoteProcessingConfig,
    pub pitch: PitchProcessingConfig,
    pub events: EventFilterConfig,
    pub structure: StructureProcessingConfig,
    pub tools: Vec<MidiModifierTool>,
    pub piano_only: bool,
    pub zero_velocity_note_on: ZeroVelocityNoteOnMode,
}

impl Default for MidiFileProcessingConfig {
    fn default() -> Self {
        Self {
            time: FileTimeProcessingConfig::default(),
            notes: NoteProcessingConfig::default(),
            pitch: PitchProcessingConfig::default(),
            events: EventFilterConfig::default(),
            structure: StructureProcessingConfig::default(),
            tools: Vec::new(),
            piano_only: false,
            zero_velocity_note_on: ZeroVelocityNoteOnMode::NoteOff,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum MidiMergeMode {
    PreserveTracks,
    FlattenToSingleTrack,
    MergeByTrackIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct FileTimeProcessingConfig {
    pub offset_ticks: i64,
    pub ppq_override: Option<u16>,
    pub tempo_override: Option<u32>,
    pub trim: Option<TrimProcessingConfig>,
}

impl Default for FileTimeProcessingConfig {
    fn default() -> Self {
        Self {
            offset_ticks: 0,
            ppq_override: None,
            tempo_override: None,
            trim: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct TrimProcessingConfig {
    pub start_tick: u64,
    pub end_tick: Option<u64>,
    pub inject_edge_state: bool,
    pub close_open_notes_at_end: bool,
}

impl Default for TrimProcessingConfig {
    fn default() -> Self {
        Self {
            start_tick: 0,
            end_tick: None,
            inject_edge_state: true,
            close_open_notes_at_end: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct PitchProcessingConfig {
    pub bend_scale: f32,
    pub bend_offset: i16,
    pub min_bend: i16,
    pub max_bend: i16,
}

impl Default for PitchProcessingConfig {
    fn default() -> Self {
        Self {
            bend_scale: 1.0,
            bend_offset: 0,
            min_bend: -8192,
            max_bend: 8191,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct StructureProcessingConfig {
    pub split_channels: bool,
    pub collapse_tracks: bool,
    pub remove_empty_tracks: bool,
    pub drop_orphan_note_offs: bool,
}

impl Default for StructureProcessingConfig {
    fn default() -> Self {
        Self {
            split_channels: false,
            collapse_tracks: false,
            remove_empty_tracks: true,
            drop_orphan_note_offs: true,
        }
    }
}
