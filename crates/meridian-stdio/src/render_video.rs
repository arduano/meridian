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
    render::{RendererKind, SceneLayout},
    spawn_core,
    video::{VideoRenderConfig, VideoRenderEvent, render_video},
};

pub fn run(
    midi: &Path,
    output: &Path,
    fps: f64,
    width: u32,
    height: u32,
    view_range: f64,
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
        view_range,
        first_key,
        last_key,
        ffmpeg_args: parse_ffmpeg_args(ffmpeg_flags)?,
    };

    let core = spawn_core();
    let mut stdout = BufWriter::new(io::stdout().lock());
    let result = render_video(&core, &config, &cancel, |event| {
        let _ = write_event(&mut stdout, &event);
    });
    let _ = core.request(meridian_core::protocol::CoreCommand::Shutdown);
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
