use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::audio::{AudioRenderEvent, AudioStatus};
use crate::render::SceneLayout;

use super::{
    commands::ImageOutputFormat,
    ids::{AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId},
    render::{AudioRenderStatus, FrameStats, VideoRenderEvent, VideoRenderStatus},
    state::StateSnapshot,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreErrorCode {
    InvalidJson,
    InvalidProtocolVersion,
    InvalidCommand,
    InvalidViewport,
    InvalidLayout,
    NoMidiLoaded,
    UnsupportedFormat,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    ParsedMidiLoaded {
        parsed_midi_id: ParsedMidiId,
        path: PathBuf,
    },
    DisplayCacheBuilt {
        parsed_midi_id: ParsedMidiId,
        display_cache_id: DisplayCacheId,
        midi_length: f64,
        total_notes: u64,
        track_count: usize,
    },
    AudioCacheBuilt {
        parsed_midi_id: ParsedMidiId,
        audio_cache_id: AudioCacheId,
        total_events: usize,
    },
    DisplaySessionCreated {
        display_session_id: DisplaySessionId,
        display_cache_id: DisplayCacheId,
    },
    AudioSessionCreated {
        audio_session_id: AudioSessionId,
        audio_cache_id: AudioCacheId,
    },
    DisplayCacheAttached {
        display_cache_id: DisplayCacheId,
        state: StateSnapshot,
    },
    AudioCacheAttached {
        audio_cache_id: AudioCacheId,
        state: StateSnapshot,
    },
    DisplaySessionAttached {
        display_session_id: DisplaySessionId,
        state: StateSnapshot,
    },
    AudioSessionAttached {
        audio_session_id: AudioSessionId,
        state: StateSnapshot,
    },
    StateSnapshot {
        state: StateSnapshot,
    },
    MidiLoaded {
        path: PathBuf,
        state: StateSnapshot,
    },
    AudioStatus {
        status: AudioStatus,
    },
    AudioRender {
        event: AudioRenderEvent,
    },
    AudioRenderStatus {
        status: AudioRenderStatus,
    },
    FrameProjected {
        state: StateSnapshot,
        layout: SceneLayout,
        stats: FrameStats,
    },
    FrameSaved {
        output: PathBuf,
        format: ImageOutputFormat,
        state: StateSnapshot,
        stats: FrameStats,
        bytes_written: u64,
    },
    VideoRender {
        event: VideoRenderEvent,
    },
    VideoRenderStatus {
        status: VideoRenderStatus,
    },
    Error {
        code: CoreErrorCode,
        message: String,
    },
    ShutdownComplete,
}
