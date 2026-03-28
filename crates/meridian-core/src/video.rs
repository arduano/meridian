use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::{
    CoreHandle, MeridianError,
    protocol::CoreCommand,
    render::{SceneConfig, pfa::wgpu::HeadlessRenderSession},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRenderConfig {
    pub midi_path: Option<PathBuf>,
    pub output: PathBuf,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub scene: Option<SceneConfig>,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VideoRenderEvent {
    RenderStarted {
        midi: Option<PathBuf>,
        output: PathBuf,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        duration_seconds: f64,
        ffmpeg_command: Vec<String>,
    },
    RenderProgress {
        frame_index: u64,
        total_frames: u64,
        current_time: f64,
        elapsed_seconds: f64,
        average_fps: f64,
    },
    RenderCancelled {
        frame_index: u64,
        total_frames: u64,
        elapsed_seconds: f64,
    },
    RenderFinished {
        total_frames: u64,
        elapsed_seconds: f64,
        average_fps: f64,
        output: PathBuf,
    },
    RenderFailed {
        message: String,
    },
}

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

    if let Some(scene) = &config.scene {
        core.request(CoreCommand::SetSceneConfig {
            scene: scene.clone(),
        })?;
    }
    core.request(CoreCommand::SetViewRange {
        seconds: config.view_range,
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: config.first_key,
        last_key: config.last_key,
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
    let mut session = HeadlessRenderSession::new(config.width, config.height)?;

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
        let frame = core.render_frame(Some(config.width), Some(config.height))?;
        session.render(&frame.scene);
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

fn spawn_ffmpeg(
    output: &Path,
    fps: f64,
    width: u32,
    height: u32,
    extra_args: &[String],
) -> Result<(Child, ChildStdin, Vec<String>), MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "-s:v".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "pipe:0".to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ];
    args.extend(extra_args.iter().cloned());
    args.push(output.display().to_string());

    let mut command = Command::new("ffmpeg");
    command
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .map_err(|e| MeridianError::Platform(format!("failed to spawn ffmpeg: {e}")))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| MeridianError::Platform("ffmpeg stdin was not available".into()))?;
    let mut printable = vec!["ffmpeg".to_string()];
    printable.extend(args);
    Ok((child, stdin, printable))
}
