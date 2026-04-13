use std::{
    io::{self, BufWriter},
    path::Path,
    sync::atomic::Ordering,
};

use meridian_core::{
    MeridianError,
    protocol::{
        FrameColorMode, ProtocolClient, ProtocolCommand, ProtocolEvent, ProtocolVideoRenderConfig,
        VideoExportConfig, VideoOutputContainer,
    },
    render::{DisplayTimeSpace, RendererKind},
};

use crate::render_common::{
    assert_no_protocol_error, install_cancel_handler, write_event, write_events,
};

#[expect(
    clippy::too_many_arguments,
    reason = "Video render CLI wiring keeps the full explicit flag surface visible at the boundary."
)]
pub fn run(
    midi: &Path,
    output: &Path,
    start_time: Option<f64>,
    end_time: Option<f64>,
    fps: f64,
    container: VideoOutputContainer,
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
    let cancel = install_cancel_handler()?;

    let client = ProtocolClient::spawn();
    let mut stdout = BufWriter::new(io::stdout().lock());
    let mut cancel_sent = false;

    let start_response = client.request(ProtocolCommand::StartRenderVideo {
        config: ProtocolVideoRenderConfig {
            midi_path: midi.to_path_buf(),
            output: output.to_path_buf(),
            container,
            fps,
            width,
            height,
            renderer: Some(renderer),
            scene: None,
            view_range: Some(view_range),
            time_space: Some(time_space),
            start_time,
            end_time,
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
    write_events(&mut stdout, &start_response.events, "video render event")?;
    assert_no_protocol_error(&start_response.events)?;

    let result = loop {
        if cancel.load(Ordering::SeqCst) && !cancel_sent {
            let response = client.request(ProtocolCommand::CancelRenderVideo)?;
            assert_no_protocol_error(&response.events)?;
            cancel_sent = true;
        }

        match client.recv()? {
            event @ ProtocolEvent::VideoRender { .. } => {
                write_event(&mut stdout, &event, "video render event")?;
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
