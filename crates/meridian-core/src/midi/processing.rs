use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EventFilterConfig {
    pub notes: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZeroVelocityNoteOnMode {
    NoteOff,
    KeepAsNoteOn,
}
