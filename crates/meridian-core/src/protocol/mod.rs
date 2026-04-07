mod analysis_jobs;
mod client;
mod commands;
mod events;
mod ids;
mod json;
mod process;
mod render;
mod server;
mod state;
mod wire;

pub use analysis_jobs::{MidiAnalysisJobEvent, MidiAnalysisJobStatus};
pub use client::ProtocolClient;
pub use commands::{CoreCommand, ImageOutputFormat};
pub use events::{CoreErrorCode, CoreEvent};
pub use ids::{
    AnalysisJobId, AudioCacheId, AudioRenderJobId, AudioSessionId, DisplayCacheId,
    DisplaySessionId, MidiProcessJobId, ParsedMidiId, ProcessedMidiId, VideoRenderJobId,
};
pub use json::{JsonRequest, JsonResponse};
pub use process::{MidiProcessEvent, MidiProcessStatus};
pub use render::{
    AudioOutputFormat, AudioRenderStatus, FrameColorMode, FrameStats, ImageExportArtifacts,
    ImageExportConfig, RenderedFrame, VideoAudioConfig, VideoAudioProgress, VideoExportArtifacts,
    VideoExportConfig, VideoRenderConfig, VideoRenderEvent, VideoRenderStatus,
};
pub use server::{run_one_protocol_json, serve_protocol_json};
pub use state::StateSnapshot;
pub use wire::{
    ProtocolAudioRenderConfig, ProtocolCommand, ProtocolEvent, ProtocolRequest, ProtocolResponse,
    ProtocolSchemaDemo, ProtocolVideoRenderConfig, UnsupportedProtocolEvent,
};

pub use crate::midi::analysis::{
    MidiAnalysisBucket, MidiAnalysisData, MidiAnalysisKind, MidiFileInspection,
};

pub const PROTOCOL_VERSION: u32 = 1;

pub const fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}
