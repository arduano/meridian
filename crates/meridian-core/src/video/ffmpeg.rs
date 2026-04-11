use std::{
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
};

use crate::{MeridianError, protocol::VideoOutputContainer};

pub struct VideoFfmpegAudioInput<'a> {
    pub pipe_path: &'a Path,
    pub sample_rate: u32,
    pub channels: u16,
    pub extra_args: &'a [String],
}

pub fn spawn_ffmpeg_rgba(
    output: &Path,
    container: VideoOutputContainer,
    fps: f64,
    width: u32,
    height: u32,
    extra_args: &[String],
    audio_input: Option<VideoFfmpegAudioInput<'_>>,
) -> Result<(Child, ChildStdin, Vec<String>), MeridianError> {
    spawn_ffmpeg(
        output,
        container,
        fps,
        width,
        height,
        "rgba",
        extra_args,
        audio_input,
    )
}

pub fn spawn_ffmpeg_gray(
    output: &Path,
    container: VideoOutputContainer,
    fps: f64,
    width: u32,
    height: u32,
    extra_args: &[String],
) -> Result<(Child, ChildStdin, Vec<String>), MeridianError> {
    spawn_ffmpeg(
        output, container, fps, width, height, "gray", extra_args, None,
    )
}

fn spawn_ffmpeg(
    output: &Path,
    container: VideoOutputContainer,
    fps: f64,
    width: u32,
    height: u32,
    input_pix_fmt: &str,
    extra_args: &[String],
    audio_input: Option<VideoFfmpegAudioInput<'_>>,
) -> Result<(Child, ChildStdin, Vec<String>), MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        input_pix_fmt.to_string(),
        "-s:v".to_string(),
        format!("{width}x{height}"),
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        "pipe:0".to_string(),
    ];
    if let Some(audio_input) = audio_input {
        args.extend([
            "-f".to_string(),
            "f32le".to_string(),
            "-ar".to_string(),
            audio_input.sample_rate.to_string(),
            "-ac".to_string(),
            audio_input.channels.to_string(),
            "-i".to_string(),
            audio_input.pipe_path.display().to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
        ]);
        args.extend(audio_input.extra_args.iter().cloned());
    } else {
        args.push("-an".to_string());
    }
    args.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
    ]);
    args.extend(extra_args.iter().cloned());
    args.extend(["-f".to_string(), container.ffmpeg_format_name().to_string()]);
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
