use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ids::MidiProcessJobId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MidiProcessEvent {
    ProcessStarted {
        job_id: MidiProcessJobId,
        output: PathBuf,
        total_inputs: usize,
    },
    InputProgress {
        job_id: MidiProcessJobId,
        processed_inputs: usize,
        total_inputs: usize,
        current_input: Option<PathBuf>,
    },
    ProcessFinished {
        job_id: MidiProcessJobId,
        output: PathBuf,
        input_count: usize,
        output_track_count: usize,
        output_ppq: u16,
        total_events: usize,
    },
    ProcessCancelled {
        job_id: MidiProcessJobId,
        output: PathBuf,
        processed_inputs: usize,
        total_inputs: usize,
    },
    ProcessFailed {
        job_id: MidiProcessJobId,
        output: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MidiProcessStatus {
    Idle,
    Running {
        job_id: MidiProcessJobId,
        output: PathBuf,
        processed_inputs: usize,
        total_inputs: usize,
        current_input: Option<PathBuf>,
    },
    Cancelling {
        job_id: MidiProcessJobId,
        output: PathBuf,
        processed_inputs: usize,
        total_inputs: usize,
        current_input: Option<PathBuf>,
    },
}
