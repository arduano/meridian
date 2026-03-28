use std::{
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
};

use crate::MeridianError;

pub fn spawn_ffmpeg(
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
