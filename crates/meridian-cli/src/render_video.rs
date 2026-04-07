use std::{
    io::{self, BufWriter, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use meridian_core::{
    MeridianError,
    protocol::{
        FrameColorMode, ProtocolClient, ProtocolCommand, ProtocolEvent, ProtocolVideoRenderConfig,
        VideoExportConfig,
    },
    render::{DisplayTimeSpace, RendererKind},
};

pub fn run(
    midi: &Path,
    output: &Path,
    fps: f64,
    width: u32,
    height: u32,
    view_range: f64,
    time_space: DisplayTimeSpace,
    first_key: u8,
    last_key: u8,
    renderer: RendererKind,
    rgb_mode: FrameColorMode,
    export_alpha_mask: bool,
    ffmpeg_flags: Option<&str>,
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

    let start_response = client.request(ProtocolCommand::StartRenderVideo {
        config: ProtocolVideoRenderConfig {
            midi_path: midi.to_path_buf(),
            output: output.to_path_buf(),
            fps,
            width,
            height,
            renderer: Some(renderer),
            scene: None,
            view_range: Some(view_range),
            time_space: Some(time_space),
            first_key: Some(first_key),
            last_key: Some(last_key),
            ffmpeg_args: parse_ffmpeg_args(ffmpeg_flags)?,
            export: VideoExportConfig {
                color_mode: rgb_mode,
                export_alpha_mask,
            },
            audio: None,
        },
    })?;
    write_events(&mut stdout, &start_response.events)?;
    assert_no_protocol_error(&start_response.events)?;

    let result = loop {
        if cancel.load(Ordering::SeqCst) && !cancel_sent {
            let response = client.request(ProtocolCommand::CancelRenderVideo)?;
            assert_no_protocol_error(&response.events)?;
            cancel_sent = true;
        }

        match client.recv()? {
            event @ ProtocolEvent::VideoRender { .. } => {
                write_event(&mut stdout, &event)?;
                match event {
                    ProtocolEvent::VideoRender { event } => match event {
                        meridian_core::protocol::VideoRenderEvent::RenderFinished { .. } => {
                            break Ok(());
                        }
                        meridian_core::protocol::VideoRenderEvent::RenderCancelled { .. } => {
                            break Err(MeridianError::Cancelled(
                                "video render was cancelled".into(),
                            ));
                        }
                        meridian_core::protocol::VideoRenderEvent::RenderFailed { message } => {
                            break Err(MeridianError::Protocol(message));
                        }
                        meridian_core::protocol::VideoRenderEvent::RenderStarted { .. }
                        | meridian_core::protocol::VideoRenderEvent::RenderProgress { .. } => {}
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

fn parse_ffmpeg_args(value: Option<&str>) -> Result<Vec<String>, MeridianError> {
    match value {
        Some(value) => shell_words::split(value)
            .map_err(|e| MeridianError::Protocol(format!("invalid ffmpeg flags: {e}"))),
        None => Ok(Vec::new()),
    }
}

fn write_events(
    stdout: &mut BufWriter<impl Write>,
    events: &[ProtocolEvent],
) -> Result<(), MeridianError> {
    for event in events {
        write_event(stdout, event)?;
    }
    Ok(())
}

fn write_event(
    stdout: &mut BufWriter<impl Write>,
    event: &ProtocolEvent,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, event).map_err(|e| {
        MeridianError::Protocol(format!("failed to serialize video render event: {e}"))
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
