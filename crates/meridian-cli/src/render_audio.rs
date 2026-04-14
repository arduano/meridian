use std::{
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use meridian_core::{
    MeridianError,
    protocol::{
        AudioOutputFormat, ProtocolAudioRenderConfig, ProtocolClient, ProtocolCommand,
        ProtocolEvent,
    },
};

use crate::render_common::{assert_no_protocol_error, install_cancel_handler, write_event};

pub fn run(
    midi: &Path,
    output: &Path,
    format: AudioOutputFormat,
    sample_rate: u32,
    channels: u16,
    use_limiter: bool,
    soundfonts: &[PathBuf],
) -> Result<(), MeridianError> {
    let cancel = install_cancel_handler()?;
    validate_audio_output_path(output, format)?;

    let client = ProtocolClient::spawn();
    let mut stdout = BufWriter::new(io::stdout().lock());
    let mut cancel_sent = false;

    let load_response = client.request(ProtocolCommand::LoadAudioMidi {
        path: midi.to_path_buf(),
    })?;
    assert_no_protocol_error(&load_response.events)?;

    let start_response = client.request(ProtocolCommand::StartRenderAudio {
        config: ProtocolAudioRenderConfig {
            midi_path: midi.to_path_buf(),
            output: output.to_path_buf(),
            sample_rate: Some(sample_rate),
            channels: Some(channels),
            use_limiter: Some(use_limiter),
            format,
            ffmpeg_args: Vec::new(),
            soundfonts: soundfonts.to_vec(),
        },
    })?;
    assert_no_protocol_error(&start_response.events)?;

    let result = loop {
        if cancel.load(Ordering::SeqCst) && !cancel_sent {
            let response = client.request(ProtocolCommand::CancelRenderAudio)?;
            assert_no_protocol_error(&response.events)?;
            cancel_sent = true;
        }

        match client.recv()? {
            event @ ProtocolEvent::AudioRender { .. } => {
                write_event(&mut stdout, &event, "audio render event")?;
                match event {
                    ProtocolEvent::AudioRender { event } => match event {
                        meridian_core::audio::AudioRenderEvent::RenderFinished { .. } => {
                            break Ok(());
                        }
                        meridian_core::audio::AudioRenderEvent::RenderCancelled { .. } => {
                            break Err(MeridianError::Cancelled(
                                "Audio render was cancelled".into(),
                            ));
                        }
                        meridian_core::audio::AudioRenderEvent::RenderFailed { message } => {
                            break Err(MeridianError::Protocol(message));
                        }
                        meridian_core::audio::AudioRenderEvent::RenderStarted { .. }
                        | meridian_core::audio::AudioRenderEvent::RenderProgress { .. } => {}
                    },
                    _ => unreachable!(),
                }
            }
            ProtocolEvent::Error { code, message } => {
                break Err(MeridianError::Protocol(format!("{code:?}: {message}")));
            }
            _ => {}
        }
    };

    let _ = client.shutdown();
    result
}

fn validate_audio_output_path(
    output: &Path,
    format: AudioOutputFormat,
) -> Result<(), MeridianError> {
    let expected_extension = match format {
        AudioOutputFormat::Wav => "wav",
        AudioOutputFormat::Flac => "flac",
        AudioOutputFormat::Mp3 => "mp3",
    };

    let Some(actual_extension) = output.extension().and_then(|extension| extension.to_str()) else {
        return Err(MeridianError::Platform(format!(
            "Audio output path must use .{expected_extension} for the selected format"
        )));
    };

    if actual_extension.eq_ignore_ascii_case(expected_extension) {
        Ok(())
    } else {
        Err(MeridianError::Platform(format!(
            "Audio output path must use .{expected_extension} for the selected format"
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use meridian_core::protocol::AudioOutputFormat;

    use super::validate_audio_output_path;

    #[test]
    fn validate_audio_output_path_accepts_matching_extensions() {
        for (path, format) in [
            (Path::new("render.wav"), AudioOutputFormat::Wav),
            (Path::new("render.flac"), AudioOutputFormat::Flac),
            (Path::new("render.mp3"), AudioOutputFormat::Mp3),
        ] {
            validate_audio_output_path(path, format).expect("matching extension should be valid");
        }
    }

    #[test]
    fn validate_audio_output_path_rejects_missing_or_mismatched_extensions() {
        for (path, format, expected_fragment) in [
            (Path::new("render"), AudioOutputFormat::Wav, ".wav"),
            (Path::new("render.wav"), AudioOutputFormat::Flac, ".flac"),
            (Path::new("render.flac"), AudioOutputFormat::Mp3, ".mp3"),
        ] {
            let error = validate_audio_output_path(path, format)
                .expect_err("mismatched extension should be rejected");
            let message = error.to_string();
            assert!(
                message.contains(expected_fragment),
                "unexpected error message: {message}"
            );
        }
    }
}
