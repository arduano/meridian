use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    audio::{AudioConfig, AudioStatus},
    render::{DisplayTimeSpace, SceneConfig},
};

use super::ids::{
    AudioCacheId, AudioRenderJobId, AudioSessionId, DisplayCacheId, DisplaySessionId,
    MidiProcessJobId, ParsedMidiId, ProcessedMidiId, VideoRenderJobId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub active_parsed_midi_id: Option<ParsedMidiId>,
    pub active_processed_midi_id: Option<ProcessedMidiId>,
    pub active_display_cache_id: Option<DisplayCacheId>,
    pub active_audio_cache_id: Option<AudioCacheId>,
    pub active_display_session_id: Option<DisplaySessionId>,
    pub active_audio_session_id: Option<AudioSessionId>,
    pub active_video_render_job_id: Option<VideoRenderJobId>,
    pub active_audio_render_job_id: Option<AudioRenderJobId>,
    pub active_midi_process_job_id: Option<MidiProcessJobId>,
    pub midi_path: Option<PathBuf>,
    pub midi_loaded: bool,
    pub audio: AudioConfig,
    pub audio_status: AudioStatus,
    pub scene: SceneConfig,
    pub current_time: f64,
    pub playing: bool,
    pub midi_length: f64,
    pub total_notes: u64,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}
