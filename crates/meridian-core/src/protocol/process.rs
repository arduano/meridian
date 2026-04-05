use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ids::MidiProcessJobId;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MidiProcessEvent {
    ProcessStarted {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
    },
    ProcessFinished {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
        output_track_count: usize,
        output_ppq: u16,
        total_events: usize,
    },
    ProcessCancelled {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
    },
    ProcessFailed {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MidiProcessStatus {
    Idle,
    Running {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
    },
    Cancelling {
        job_id: MidiProcessJobId,
        input: PathBuf,
        output: PathBuf,
    },
}
