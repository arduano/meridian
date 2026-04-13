use crate::{
    error::MeridianError,
    protocol::{CoreErrorCode, CoreEvent, ImageOutputFormat},
};

pub fn error_event(code: CoreErrorCode, message: impl Into<String>) -> CoreEvent {
    CoreEvent::Error {
        code,
        message: message.into(),
    }
}

pub fn event_to_error(context: &str, event: &CoreEvent) -> MeridianError {
    match event {
        CoreEvent::Error { code, message } => match code {
            CoreErrorCode::InvalidJson
            | CoreErrorCode::InvalidProtocolVersion
            | CoreErrorCode::InvalidCommand
            | CoreErrorCode::InvalidRequest => {
                MeridianError::Protocol(format!("{context}: {message}"))
            }
            CoreErrorCode::InvalidState
            | CoreErrorCode::ValidationFailed
            | CoreErrorCode::InvalidViewport
            | CoreErrorCode::InvalidLayout => {
                MeridianError::Validation(format!("{context}: {message}"))
            }
            CoreErrorCode::NoMidiLoaded | CoreErrorCode::ResourceNotFound => {
                MeridianError::NotFound(format!("{context}: {message}"))
            }
            CoreErrorCode::Conflict => MeridianError::Conflict(format!("{context}: {message}")),
            CoreErrorCode::Unsupported | CoreErrorCode::UnsupportedFormat => {
                MeridianError::Unsupported(format!("{context}: {message}"))
            }
            CoreErrorCode::Io => {
                MeridianError::Io(std::io::Error::other(format!("{context}: {message}")))
            }
            CoreErrorCode::Transport => MeridianError::Transport(format!("{context}: {message}")),
            CoreErrorCode::Backend => MeridianError::Backend(format!("{context}: {message}")),
            CoreErrorCode::ExternalTool => {
                MeridianError::ExternalTool(format!("{context}: {message}"))
            }
            CoreErrorCode::Cancelled => MeridianError::Cancelled(format!("{context}: {message}")),
            CoreErrorCode::Internal => MeridianError::Platform(format!("{context}: {message}")),
        },
        _ => MeridianError::Protocol(context.into()),
    }
}

pub fn error_code(error: &MeridianError) -> CoreErrorCode {
    match error {
        MeridianError::Transport(_) => CoreErrorCode::Transport,
        MeridianError::Protocol(_) => CoreErrorCode::InvalidRequest,
        MeridianError::Validation(_) => CoreErrorCode::ValidationFailed,
        MeridianError::NotFound(_) => CoreErrorCode::ResourceNotFound,
        MeridianError::Conflict(_) => CoreErrorCode::Conflict,
        MeridianError::Unsupported(_) => CoreErrorCode::Unsupported,
        MeridianError::Backend(_) => CoreErrorCode::Backend,
        MeridianError::ExternalTool(_) => CoreErrorCode::ExternalTool,
        MeridianError::Cancelled(_) => CoreErrorCode::Cancelled,
        MeridianError::InvalidMidi(message) if message.contains("no midi loaded") => {
            CoreErrorCode::NoMidiLoaded
        }
        MeridianError::InvalidMidi(message)
            if message.contains("viewport_width") || message.contains("viewport_height") =>
        {
            CoreErrorCode::InvalidViewport
        }
        MeridianError::InvalidMidi(_) => CoreErrorCode::ValidationFailed,
        MeridianError::Io(_) => CoreErrorCode::Io,
        MeridianError::Platform(_) | MeridianError::SlintNotifier(_) => CoreErrorCode::Internal,
        MeridianError::MidiLoad(_) => CoreErrorCode::ValidationFailed,
        MeridianError::Wgpu(_) => CoreErrorCode::Backend,
    }
}

impl ImageOutputFormat {
    pub fn infer_from_path(path: &std::path::Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("png") => Self::Png,
            Some("rgba") | Some("raw") => Self::Rgba,
            Some("ppm") => Self::Ppm,
            _ => Self::Ppm,
        }
    }
}
