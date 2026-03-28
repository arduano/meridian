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
        CoreEvent::Error { message, .. } => {
            MeridianError::InvalidMidi(format!("{context}: {message}"))
        }
        _ => MeridianError::InvalidMidi(context.into()),
    }
}

pub fn error_code(error: &MeridianError) -> CoreErrorCode {
    match error {
        MeridianError::InvalidMidi(message) if message.contains("no midi loaded") => {
            CoreErrorCode::NoMidiLoaded
        }
        MeridianError::InvalidMidi(message)
            if message.contains("viewport_width") || message.contains("viewport_height") =>
        {
            CoreErrorCode::InvalidViewport
        }
        MeridianError::InvalidMidi(_) => CoreErrorCode::InvalidCommand,
        MeridianError::Io(_) | MeridianError::Platform(_) | MeridianError::SlintNotifier(_) => {
            CoreErrorCode::Internal
        }
        MeridianError::MidiLoad(_) => CoreErrorCode::InvalidCommand,
        MeridianError::Wgpu(_) => CoreErrorCode::Internal,
    }
}

impl ImageOutputFormat {
    pub fn infer_from_path(path: &std::path::Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("png") => Self::Png,
            Some("rgba") | Some("raw") => Self::Rgba,
            Some("ppm") | _ => Self::Ppm,
        }
    }
}
