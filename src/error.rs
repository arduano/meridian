use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeridianError {
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("slint platform error: {0}")]
    SlintPlatform(#[from] slint::PlatformError),

    #[error("slint rendering notifier error: {0}")]
    SlintNotifier(String),

    #[error("midi load error: {0}")]
    MidiLoad(String),

    #[error("invalid midi: {0}")]
    InvalidMidi(String),
}
