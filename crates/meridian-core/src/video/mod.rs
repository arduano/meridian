mod ffmpeg;

use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::{
    CoreHandle, MeridianError,
    protocol::{CoreCommand, CoreEvent, VideoRenderConfig, VideoRenderEvent},
    render::headless::HeadlessRenderSession,
};

pub use ffmpeg::spawn_ffmpeg;

impl VideoRenderConfig {
    pub fn validate(&self) -> Result<(), MeridianError> {
        if self.fps <= 0.0 {
            return Err(MeridianError::InvalidMidi("fps must be > 0".into()));
        }
        if self.width == 0 || self.height == 0 {
            return Err(MeridianError::InvalidMidi(
                "video width and height must be > 0".into(),
            ));
        }
        Ok(())
    }
}

pub fn render_video(
    core: &CoreHandle,
    config: &VideoRenderConfig,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(VideoRenderEvent),
) -> Result<(), MeridianError> {
    if let Err(error) = render_video_inner(core, config, cancel, &mut on_event) {
        on_event(VideoRenderEvent::RenderFailed {
            message: error.to_string(),
        });
        return Err(error);
    }
    Ok(())
}

fn render_video_inner(
    core: &CoreHandle,
    config: &VideoRenderConfig,
    cancel: &AtomicBool,
    on_event: &mut impl FnMut(VideoRenderEvent),
) -> Result<(), MeridianError> {
    config.validate()?;

    let state = match core.request(CoreCommand::GetState)? {
        events => match events.into_iter().next() {
            Some(CoreEvent::StateSnapshot { state }) => state,
            _ => {
                return Err(MeridianError::Platform(
                    "unexpected response while reading core state".into(),
                ));
            }
        },
    };

    if let Some(scene) = &config.scene {
        core.request(CoreCommand::SetSceneConfig {
            scene: scene.clone(),
        })?;
    }
    core.request(CoreCommand::SetViewRange {
        seconds: config.view_range.unwrap_or(state.view_range),
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: config.first_key.unwrap_or(state.first_key),
        last_key: config.last_key.unwrap_or(state.last_key),
    })?;
    core.request(CoreCommand::SetViewport {
        width: config.width,
        height: config.height,
    })?;
    if let Some(path) = &config.midi_path {
        core.request(CoreCommand::LoadMidi { path: path.clone() })?;
    }

    let frame0 = core.render_frame(Some(config.width), Some(config.height))?;
    let duration_seconds = frame0.state.midi_length.max(0.0);
    let total_frames = (duration_seconds * config.fps).ceil().max(1.0) as u64;
    let (mut ffmpeg_child, mut ffmpeg_stdin, ffmpeg_command) = spawn_ffmpeg(
        &config.output,
        config.fps,
        config.width,
        config.height,
        &config.ffmpeg_args,
    )?;
    let mut session = HeadlessRenderSession::new(&frame0.layout, config.width, config.height)?;

    on_event(VideoRenderEvent::RenderStarted {
        midi: config.midi_path.clone(),
        output: config.output.clone(),
        fps: config.fps,
        width: config.width,
        height: config.height,
        total_frames,
        duration_seconds,
        ffmpeg_command,
    });

    let start = Instant::now();
    core.request(CoreCommand::ResetProjectorPhysics)?;
    for frame_index in 0..total_frames {
        if cancel.load(Ordering::SeqCst) {
            drop(ffmpeg_stdin);
            let _ = ffmpeg_child.kill();
            let _ = ffmpeg_child.wait();
            on_event(VideoRenderEvent::RenderCancelled {
                frame_index,
                total_frames,
                elapsed_seconds: start.elapsed().as_secs_f64(),
            });
            return Ok(());
        }

        let current_time = frame_index as f64 / config.fps;
        core.request(CoreCommand::SetTime { time: current_time })?;
        core.request(CoreCommand::TickProjectorPhysics {
            delta_seconds: 1.0 / config.fps,
        })?;
        let frame = core.render_frame(Some(config.width), Some(config.height))?;
        session.render(&frame.layout, &frame.scene);
        let rgba = session.readback_rgba()?;
        ffmpeg_stdin.write_all(&rgba)?;

        let elapsed_seconds = start.elapsed().as_secs_f64();
        let average_fps = if elapsed_seconds > 0.0 {
            (frame_index + 1) as f64 / elapsed_seconds
        } else {
            0.0
        };
        on_event(VideoRenderEvent::RenderProgress {
            frame_index: frame_index + 1,
            total_frames,
            current_time,
            elapsed_seconds,
            average_fps,
        });
    }

    drop(ffmpeg_stdin);
    let status = ffmpeg_child.wait()?;
    if !status.success() {
        return Err(MeridianError::Platform(format!(
            "ffmpeg exited with status {status}"
        )));
    }

    let elapsed_seconds = start.elapsed().as_secs_f64();
    let average_fps = if elapsed_seconds > 0.0 {
        total_frames as f64 / elapsed_seconds
    } else {
        0.0
    };
    on_event(VideoRenderEvent::RenderFinished {
        total_frames,
        elapsed_seconds,
        average_fps,
        output: config.output.clone(),
    });
    Ok(())
}
