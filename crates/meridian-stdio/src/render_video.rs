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
    protocol::{CoreCommand, CoreEvent, VideoRenderConfig, VideoRenderEvent, VideoRenderStatus},
    render::{DisplayTimeSpace, RendererKind, SceneLayout},
    spawn_core,
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

    let mut layout = SceneLayout::default();
    layout.set_renderer_kind(renderer);
    let config = VideoRenderConfig {
        midi_path: Some(midi.to_path_buf()),
        output: output.to_path_buf(),
        fps,
        width,
        height,
        scene: Some(layout.scene),
        view_range: Some(view_range),
        time_space: Some(time_space),
        first_key: Some(first_key),
        last_key: Some(last_key),
        ffmpeg_args: parse_ffmpeg_args(ffmpeg_flags)?,
    };

    let core = spawn_core();
    let event_rx = core.subscribe_events();
    let mut stdout = BufWriter::new(io::stdout().lock());
    write_core_events(
        &mut stdout,
        &core.request(CoreCommand::StartRenderVideo { config })?,
    )?;
    let mut terminal_result: Option<Result<(), MeridianError>> = None;
    let result = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = core.request(CoreCommand::CancelRenderVideo);
        }

        let event = event_rx
            .recv()
            .map_err(|_| MeridianError::Wgpu("core event channel closed".into()))?;
        match &event {
            CoreEvent::VideoRender { event } => {
                write_event(&mut stdout, event)?;
                match event {
                    VideoRenderEvent::RenderFinished { .. }
                    | VideoRenderEvent::RenderCancelled { .. } => terminal_result = Some(Ok(())),
                    VideoRenderEvent::RenderFailed { message } => {
                        terminal_result = Some(Err(MeridianError::Platform(message.clone())));
                    }
                    VideoRenderEvent::RenderStarted { .. }
                    | VideoRenderEvent::RenderProgress { .. } => {}
                }
            }
            CoreEvent::VideoRenderStatus { status } => {
                if matches!(status, VideoRenderStatus::Idle) {
                    if let Some(result) = terminal_result.take() {
                        break result;
                    }
                }
            }
            _ => {}
        }
    };
    let _ = core.request(CoreCommand::Shutdown);
    result
}

fn parse_ffmpeg_args(value: Option<&str>) -> Result<Vec<String>, MeridianError> {
    match value {
        Some(value) => shell_words::split(value)
            .map_err(|e| MeridianError::Platform(format!("invalid ffmpeg flags: {e}"))),
        None => Ok(Vec::new()),
    }
}

fn write_event(
    stdout: &mut BufWriter<impl Write>,
    event: &VideoRenderEvent,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, event)
        .map_err(|e| MeridianError::Platform(format!("failed to serialize progress event: {e}")))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn write_core_events(
    stdout: &mut BufWriter<impl Write>,
    events: &[CoreEvent],
) -> Result<(), MeridianError> {
    for event in events {
        serde_json::to_writer(&mut *stdout, event)
            .map_err(|e| MeridianError::Platform(format!("failed to serialize core event: {e}")))?;
        stdout.write_all(b"\n")?;
    }
    stdout.flush()?;
    Ok(())
}
