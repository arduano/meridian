use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::midi::analysis::{MidiAnalysisData, MidiAnalysisKind};

use super::ids::{AnalysisJobId, ParsedMidiId};

/// Streamed events emitted by the full analysis pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MidiAnalysisJobEvent {
    Started {
        job_id: AnalysisJobId,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
    },
    Progress {
        job_id: AnalysisJobId,
        progress: f32,
        status: String,
    },
    Finished {
        job_id: AnalysisJobId,
        result: MidiAnalysisData,
    },
    Failed {
        job_id: AnalysisJobId,
        message: String,
    },
}

/// Materialized job state derived from `MidiAnalysisJobEvent`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MidiAnalysisJobStatus {
    Running {
        job_id: AnalysisJobId,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
        progress: f32,
        status: String,
    },
    Finished {
        job_id: AnalysisJobId,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
        result: MidiAnalysisData,
    },
    Failed {
        job_id: AnalysisJobId,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
        message: String,
    },
}
