use std::{
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin},
};

use crate::{MeridianError, protocol::VideoRenderConfig};

use super::{audio_mux::VideoAudioMux, ffmpeg::spawn_ffmpeg_gray};

pub(crate) struct EncoderProcess {
    name: &'static str,
    child: Child,
    stdin: Option<ChildStdin>,
}

impl EncoderProcess {
    pub(crate) fn new(name: &'static str, child: Child, stdin: ChildStdin) -> Self {
        Self {
            name,
            child,
            stdin: Some(stdin),
        }
    }

    pub(crate) fn stdin_mut(&mut self) -> Option<&mut ChildStdin> {
        self.stdin.as_mut()
    }

    pub(crate) fn close_stdin(&mut self) {
        let _ = self.stdin.take();
    }

    pub(crate) fn kill_and_wait(&mut self) {
        self.close_stdin();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    pub(crate) fn wait_for_success(mut self) -> Result<(), MeridianError> {
        self.close_stdin();
        let status = self.child.wait()?;
        if !status.success() {
            return Err(MeridianError::Platform(format!(
                "{} exited with status {status}",
                self.name
            )));
        }
        Ok(())
    }
}

pub(crate) struct VideoRenderPipeline {
    ffmpeg: Option<EncoderProcess>,
    alpha_ffmpeg: Option<EncoderProcess>,
    audio_mux: Option<VideoAudioMux>,
}

impl VideoRenderPipeline {
    pub(crate) fn new(
        ffmpeg_child: Child,
        ffmpeg_stdin: ChildStdin,
        audio_mux: Option<VideoAudioMux>,
    ) -> Self {
        Self {
            ffmpeg: Some(EncoderProcess::new("ffmpeg", ffmpeg_child, ffmpeg_stdin)),
            alpha_ffmpeg: None,
            audio_mux,
        }
    }

    pub(crate) fn set_alpha_ffmpeg(&mut self, alpha_ffmpeg: Option<EncoderProcess>) {
        self.alpha_ffmpeg = alpha_ffmpeg;
    }

    pub(crate) fn has_alpha_ffmpeg(&self) -> bool {
        self.alpha_ffmpeg.is_some()
    }

    pub(crate) fn start_audio_mux(&mut self) -> Result<(), MeridianError> {
        if let Some(audio_mux) = self.audio_mux.as_mut() {
            audio_mux.start()?;
        }
        Ok(())
    }

    pub(crate) fn audio_progress(&self) -> Option<crate::protocol::VideoAudioProgress> {
        self.audio_mux.as_ref().map(VideoAudioMux::progress)
    }

    pub(crate) fn write_color_frame(&mut self, frame: &[u8]) -> Result<(), MeridianError> {
        let stdin = self
            .ffmpeg
            .as_mut()
            .and_then(EncoderProcess::stdin_mut)
            .ok_or_else(|| MeridianError::Platform("ffmpeg stdin is not available".into()))?;
        stdin.write_all(frame)?;
        Ok(())
    }

    pub(crate) fn write_alpha_frame(&mut self, frame: &[u8]) -> Result<(), MeridianError> {
        let Some(alpha_ffmpeg) = self.alpha_ffmpeg.as_mut() else {
            return Ok(());
        };
        let stdin = alpha_ffmpeg
            .stdin_mut()
            .ok_or_else(|| MeridianError::Platform("alpha ffmpeg stdin is not available".into()))?;
        stdin.write_all(frame)?;
        Ok(())
    }

    pub(crate) fn cancel(&mut self) {
        if let Some(mut alpha_ffmpeg) = self.alpha_ffmpeg.take() {
            alpha_ffmpeg.kill_and_wait();
        }
        if let Some(mut ffmpeg) = self.ffmpeg.take() {
            ffmpeg.kill_and_wait();
        }
        if let Some(audio_mux) = self.audio_mux.as_mut() {
            audio_mux.cancel_and_join();
        }
        let _ = self.audio_mux.take();
    }

    pub(crate) fn finish(mut self) -> Result<(), MeridianError> {
        if let Some(ffmpeg) = self.ffmpeg.take() {
            ffmpeg.wait_for_success()?;
        }
        if let Some(alpha_ffmpeg) = self.alpha_ffmpeg.take() {
            alpha_ffmpeg.wait_for_success()?;
        }
        if let Some(audio_mux) = self.audio_mux.take() {
            audio_mux.finish()?;
        }
        Ok(())
    }
}

impl Drop for VideoRenderPipeline {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(crate) fn spawn_alpha_ffmpeg(
    alpha_output: Option<PathBuf>,
    config: &VideoRenderConfig,
) -> Result<(Option<EncoderProcess>, Option<Vec<String>>), MeridianError> {
    let Some(alpha_output) = alpha_output else {
        return Ok((None, None));
    };

    let (child, stdin, command) = spawn_ffmpeg_gray(
        &alpha_output,
        config.container,
        config.fps,
        config.width,
        config.height,
        &config.ffmpeg_args,
    )?;
    Ok((
        Some(EncoderProcess::new("alpha ffmpeg", child, stdin)),
        Some(command),
    ))
}
