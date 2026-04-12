use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::AtomicBool,
};

use crate::{
    MeridianError,
    ffmpeg,
    midi::audio_cache::InRamAudioCache,
    protocol::{AudioOutputFormat, AudioRenderJobId},
};

use super::{
    AudioConfig,
    config::{AudioRenderConfig, ResolvedAudioRenderSettings},
    events::AudioRenderEvent,
    render_loop::{AudioRenderLoopResult, run_audio_render_loop},
    renderer::OfflineAudioRenderer,
};

use super::super::soundfont_cache::SoundfontCache;

pub(crate) fn render_encoded_audio(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    #[cfg(unix)]
    {
        return render_encoded_audio_unix(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            callback,
        );
    }

    #[cfg(not(unix))]
    {
        return render_encoded_audio_fallback(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            callback,
        );
    }
}

#[cfg(unix)]
fn render_encoded_audio_unix(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let fifo = ffmpeg::FifoGuard::create(&render_config.output, "audio")?;
    let audio_params = settings.audio_params()?;
    let mut encoder = spawn_audio_encoder(
        &render_config.output,
        render_config.format,
        settings.sample_rate,
        settings.channels,
        &render_config.ffmpeg_args,
        fifo.path(),
    )?;
    let renderer = match OfflineAudioRenderer::new_raw_pipe(
        audio_config,
        soundfont_cache,
        fifo.path(),
        audio_params,
        settings.use_limiter,
    ) {
        Ok(renderer) => renderer,
        Err(error) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            return Err(error);
        }
    };
    let result = run_audio_render_loop(
        events,
        audio_config,
        renderer,
        render_config,
        job_id,
        cancel,
        callback,
    );
    match result {
        Ok(AudioRenderLoopResult::Finished {
            frames_written,
            rendered_seconds,
        }) => {
            let status = encoder.wait()?;
            if !status.success() {
                return Err(MeridianError::Platform(format!(
                    "ffmpeg exited with status {status}"
                )));
            }
            Ok(AudioRenderLoopResult::Finished {
                frames_written,
                rendered_seconds,
            })
        }
        Ok(AudioRenderLoopResult::Cancelled) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            Ok(AudioRenderLoopResult::Cancelled)
        }
        Err(error) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            Err(error)
        }
    }
}

#[cfg(not(unix))]
fn render_encoded_audio_fallback(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let temp_output = ffmpeg::next_temp_path(&render_config.output, "audio", "wav");
    let temp_guard = ffmpeg::TempPathGuard::new(temp_output.clone());
    let renderer = OfflineAudioRenderer::new_wav(
        audio_config,
        soundfont_cache,
        &temp_output,
        settings.audio_params()?,
        settings.use_limiter,
    )?;
    let result = run_audio_render_loop(
        events,
        audio_config,
        renderer,
        render_config,
        job_id,
        cancel,
        callback,
    )?;
    match result {
        AudioRenderLoopResult::Finished {
            frames_written,
            rendered_seconds,
        } => {
            encode_audio_file(
                &temp_output,
                &render_config.output,
                render_config.format,
                &render_config.ffmpeg_args,
            )?;
            drop(temp_guard);
            Ok(AudioRenderLoopResult::Finished {
                frames_written,
                rendered_seconds,
            })
        }
        AudioRenderLoopResult::Cancelled => Ok(AudioRenderLoopResult::Cancelled),
    }
}

fn spawn_audio_encoder(
    output: &Path,
    format: AudioOutputFormat,
    sample_rate: u32,
    channels: u16,
    extra_args: &[String],
    input: &Path,
) -> Result<Child, MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "f32le".to_string(),
        "-ar".to_string(),
        sample_rate.to_string(),
        "-ac".to_string(),
        channels.to_string(),
        "-i".to_string(),
        input.display().to_string(),
    ];
    match format {
        AudioOutputFormat::Wav => {}
        AudioOutputFormat::Flac => {}
        AudioOutputFormat::Mp3 => {
            args.extend([
                "-codec:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
            ]);
        }
    }
    args.extend(extra_args.iter().cloned());
    args.push(output.display().to_string());

    Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| MeridianError::Platform(format!("failed to spawn ffmpeg: {error}")))
}

#[cfg(not(unix))]
fn encode_audio_file(
    input: &Path,
    output: &Path,
    format: AudioOutputFormat,
    extra_args: &[String],
) -> Result<(), MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input.display().to_string(),
    ];
    match format {
        AudioOutputFormat::Wav => {}
        AudioOutputFormat::Flac => {}
        AudioOutputFormat::Mp3 => {
            args.extend([
                "-codec:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
            ]);
        }
    }
    args.extend(extra_args.iter().cloned());
    args.push(output.display().to_string());
    let status = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| MeridianError::Platform(format!("failed to spawn ffmpeg: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(MeridianError::Platform(format!(
            "ffmpeg exited with status {status}"
        )))
    }
}
