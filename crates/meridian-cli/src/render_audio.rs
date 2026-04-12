use std::{
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use meridian_core::{
    MeridianError,
    protocol::{ProtocolAudioRenderConfig, ProtocolClient, ProtocolCommand, ProtocolEvent},
};

use crate::render_common::{
    assert_no_protocol_error,
    install_cancel_handler,
    write_event,
};

pub fn run(
    midi: &Path,
    output: &Path,
    sample_rate: u32,
    channels: u16,
    use_limiter: bool,
    soundfonts: &[PathBuf],
) -> Result<(), MeridianError> {
    let cancel = install_cancel_handler()?;

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
            format: meridian_core::protocol::AudioOutputFormat::Wav,
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
                                "audio render was cancelled".into(),
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
