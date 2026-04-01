use std::{
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use meridian_core::{
    MeridianError,
    protocol::{ProtocolAudioRenderConfig, ProtocolClient, ProtocolCommand, ProtocolEvent},
};

pub fn run(
    midi: &Path,
    output: &Path,
    sample_rate: u32,
    channels: u16,
    use_limiter: bool,
    soundfonts: &[PathBuf],
) -> Result<(), MeridianError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel = Arc::clone(&cancel);
        ctrlc::set_handler(move || {
            cancel.store(true, Ordering::SeqCst);
        })
        .map_err(|e| MeridianError::Platform(format!("failed to install ctrl-c handler: {e}")))?;
    }

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
                write_event(&mut stdout, &event)?;
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

fn write_event(
    stdout: &mut BufWriter<impl Write>,
    event: &ProtocolEvent,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, event).map_err(|e| {
        MeridianError::Protocol(format!("failed to serialize audio render event: {e}"))
    })?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn assert_no_protocol_error(events: &[ProtocolEvent]) -> Result<(), MeridianError> {
    if let Some(ProtocolEvent::Error { code, message }) = events
        .iter()
        .find(|event| matches!(event, ProtocolEvent::Error { .. }))
    {
        return Err(MeridianError::Protocol(format!("{code:?}: {message}")));
    }
    Ok(())
}
