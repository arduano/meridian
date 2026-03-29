mod commands;
mod events;
mod ids;
mod json;
mod render;
mod state;

pub use commands::{CoreCommand, ImageOutputFormat};
pub use events::{CoreErrorCode, CoreEvent};
pub use ids::{
    AudioCacheId, AudioRenderJobId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId,
    ProcessedMidiId, VideoRenderJobId,
};
pub use json::{JsonRequest, JsonResponse};
pub use render::{
    AudioRenderStatus, FrameStats, RenderedFrame, VideoRenderConfig, VideoRenderEvent,
    VideoRenderStatus,
};
pub use state::StateSnapshot;

pub use crate::midi::analysis::{MidiAnalysisBucket, MidiAnalysisData};

pub const PROTOCOL_VERSION: u32 = 1;

pub const fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}
