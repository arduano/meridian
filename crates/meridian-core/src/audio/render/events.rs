use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::protocol::AudioRenderJobId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioRenderEvent {
    RenderStarted {
        job_id: AudioRenderJobId,
        output: PathBuf,
        sample_rate: u32,
        channels: u16,
        total_events: usize,
    },
    RenderProgress {
        job_id: AudioRenderJobId,
        event_index: usize,
        total_events: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
        voice_count: u64,
    },
    RenderFinished {
        job_id: AudioRenderJobId,
        output: PathBuf,
        frames_written: u64,
        rendered_seconds: f64,
    },
    RenderCancelled {
        job_id: AudioRenderJobId,
        output: PathBuf,
        event_index: usize,
        total_events: usize,
        rendered_seconds: f64,
        frames_written: u64,
    },
    RenderFailed {
        message: String,
    },
}
