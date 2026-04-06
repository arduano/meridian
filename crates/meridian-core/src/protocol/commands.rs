use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{
    audio::{AudioConfig, AudioRenderConfig},
    midi::{
        MidiFileProcessingConfig, MidiFilesMergeConfig, MidiProcessingConfig,
        analysis::MidiAnalysisKind,
    },
    render::{DisplayTimeSpace, SceneConfig},
};

use super::{
    ids::{
        AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId,
        ProcessedMidiId,
    },
    render::VideoRenderConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreCommand {
    GetState,
    LoadParsedMidi {
        path: PathBuf,
    },
    InspectMidiFiles {
        paths: Vec<PathBuf>,
    },
    BuildProcessedMidi {
        parsed_midi_id: ParsedMidiId,
        config: MidiProcessingConfig,
    },
    ProcessMidiFile {
        input: PathBuf,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    },
    MergeMidiFiles {
        inputs: Vec<PathBuf>,
        output: PathBuf,
        config: MidiFilesMergeConfig,
    },
    StartProcessMidiFile {
        input: PathBuf,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    },
    CancelMidiFileProcess,
    GetMidiFileProcessStatus,
    AnalyzeActiveMidi {
        bucket_count: Option<usize>,
    },
    AnalyzeProcessedMidi {
        processed_midi_id: ProcessedMidiId,
        bucket_count: Option<usize>,
    },
    StartMidiAnalysisJob {
        parsed_midi_id: ParsedMidiId,
        #[serde(default)]
        display_cache_id: Option<DisplayCacheId>,
        #[serde(default)]
        kinds: Vec<MidiAnalysisKind>,
        #[serde(default)]
        bucket_count: Option<usize>,
    },
    GetMidiAnalysisJobStatus {
        job_id: super::ids::AnalysisJobId,
    },
    BuildDisplayCache {
        parsed_midi_id: ParsedMidiId,
    },
    BuildAudioCache {
        parsed_midi_id: ParsedMidiId,
    },
    CreateDisplaySession {
        display_cache_id: DisplayCacheId,
    },
    CreateAudioSession {
        audio_cache_id: AudioCacheId,
    },
    AttachDisplayCache {
        display_cache_id: DisplayCacheId,
    },
    AttachProcessedMidi {
        processed_midi_id: ProcessedMidiId,
    },
    AttachAudioCache {
        audio_cache_id: AudioCacheId,
    },
    AttachDisplaySession {
        display_session_id: DisplaySessionId,
    },
    AttachAudioSession {
        audio_session_id: AudioSessionId,
    },
    LoadDisplayMidi {
        path: PathBuf,
    },
    LoadAudioMidi {
        path: PathBuf,
    },
    UnloadDisplayContext,
    UnloadAudioContext,
    UnloadRenderContext,
    DropInactiveMidiResources,
    LoadMidi {
        path: PathBuf,
    },
    SetAudioConfig {
        config: AudioConfig,
    },
    GetAudioStatus,
    StartRenderAudio {
        config: AudioRenderConfig,
    },
    CancelRenderAudio,
    GetRenderAudioStatus,
    SetTime {
        time: f64,
    },
    TickProjectorPhysics {
        delta_seconds: f64,
    },
    ResetProjectorPhysics,
    StepTime {
        delta: f64,
    },
    SetPlaying {
        playing: bool,
    },
    TogglePlaying,
    SetSceneConfig {
        scene: SceneConfig,
    },
    SetViewRange {
        seconds: f64,
        #[serde(default)]
        time_space: Option<DisplayTimeSpace>,
    },
    SetKeyRange {
        first_key: u8,
        last_key: u8,
    },
    SetViewport {
        width: u32,
        height: u32,
    },
    RenderFrame {
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    },
    SaveFrame {
        output: PathBuf,
        format: Option<ImageOutputFormat>,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    },
    StartRenderVideo {
        config: VideoRenderConfig,
    },
    CancelRenderVideo,
    GetRenderVideoStatus,
    Shutdown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ImageOutputFormat {
    Ppm,
    Png,
    Rgba,
}
