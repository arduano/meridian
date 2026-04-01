use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeridianError {
    #[error("transport error: {0}")]
    Transport(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("resource not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("external tool error: {0}")]
    ExternalTool(String),

    #[error("cancelled: {0}")]
    Cancelled(String),

    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("slint rendering notifier error: {0}")]
    SlintNotifier(String),

    #[error("midi load error: {0}")]
    MidiLoad(String),

    #[error("invalid midi: {0}")]
    InvalidMidi(String),

    #[error("wgpu error: {0}")]
    Wgpu(String),
}
