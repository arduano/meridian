//! Muxed audio worker lifecycle.
//!
//! This module owns the "render audio on the side while ffmpeg consumes the
//! video pipe" path for video exports with audio.

use std::{sync::Arc, sync::atomic::AtomicBool};

#[cfg(any(unix, windows))]
use std::{
    sync::Mutex,
    sync::atomic::Ordering,
    thread::{self, JoinHandle},
};

#[cfg(any(unix, windows))]
use crate::{
    MeridianError,
    audio::{
        RawAudioPipeTarget, SoundfontCache, render_audio_pipe_from_cache,
        resolve_video_audio_settings,
    },
    midi::audio_cache::InRamAudioCache,
    protocol::{VideoAudioConfig, VideoAudioProgress, VideoRenderConfig},
};

#[cfg(all(not(unix), not(windows)))]
use crate::{
    MeridianError,
    protocol::{VideoAudioConfig, VideoAudioProgress, VideoRenderConfig},
};

use super::{
    config::{ResolvedVideoTimeRange, VideoRenderAudioInputs},
    ffmpeg::VideoFfmpegAudioInput,
};

pub(crate) enum VideoAudioMux {
    #[cfg(any(unix, windows))]
    Pipe {
        pipe: crate::ffmpeg::AudioPipeGuard,
        sample_rate: u32,
        channels: u16,
        cancel: Arc<AtomicBool>,
        progress: Arc<Mutex<VideoAudioProgress>>,
        worker: Option<JoinHandle<Result<(), MeridianError>>>,
        inputs: Option<VideoRenderAudioInputs>,
        audio: VideoAudioConfig,
    },
    #[cfg_attr(all(not(unix), not(windows)), allow(dead_code))]
    #[cfg(all(not(unix), not(windows)))]
    Unsupported,
}

impl VideoAudioMux {
    pub(crate) fn spawn(
        config: &VideoRenderConfig,
        audio: &VideoAudioConfig,
        audio_inputs: Option<VideoRenderAudioInputs>,
        time_range: ResolvedVideoTimeRange,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Self, MeridianError> {
        #[cfg(any(unix, windows))]
        {
            // The audio mux only exists for the video render path, but it is
            // still split out so the main video orchestration stays readable.
            let inputs = audio_inputs.ok_or_else(|| {
                MeridianError::Platform("missing cached audio data for muxed video render".into())
            })?;
            let (sample_rate, channels, _) =
                resolve_video_audio_settings(&inputs.audio_config, audio)?;
            let clipped_audio_cache = clip_audio_cache(&inputs.audio_cache, time_range);
            let total_events = clipped_audio_cache.events().len();
            let pipe = crate::ffmpeg::AudioPipeGuard::create(&config.output, "audio")?;
            let _ = cancel;
            Ok(Self::Pipe {
                pipe,
                sample_rate,
                channels,
                cancel: Arc::clone(cancel),
                progress: Arc::new(Mutex::new(VideoAudioProgress {
                    total_events,
                    event_index: 0,
                    rendered_seconds: 0.0,
                })),
                worker: None,
                inputs: Some(VideoRenderAudioInputs {
                    audio_cache: Arc::new(clipped_audio_cache),
                    audio_config: inputs.audio_config,
                }),
                audio: audio.clone(),
            })
        }

        #[cfg(all(not(unix), not(windows)))]
        {
            let _ = (config, audio, audio_inputs, time_range, cancel);
            Err(MeridianError::Unsupported(
                "muxed audio video export currently requires a supported local pipe implementation"
                    .into(),
            ))
        }
    }

    pub(crate) fn ffmpeg_input(&self) -> Result<VideoFfmpegAudioInput<'_>, MeridianError> {
        match self {
            #[cfg(any(unix, windows))]
            Self::Pipe {
                pipe,
                sample_rate,
                channels,
                audio,
                ..
            } => Ok(VideoFfmpegAudioInput {
                pipe_path: pipe.path(),
                sample_rate: *sample_rate,
                channels: *channels,
                extra_args: &audio.ffmpeg_args,
            }),
            #[cfg(all(not(unix), not(windows)))]
            Self::Unsupported => Err(MeridianError::Unsupported(
                "muxed audio video export currently requires a supported local pipe implementation"
                    .into(),
            )),
        }
    }

    pub(crate) fn start(&mut self) -> Result<(), MeridianError> {
        match self {
            #[cfg(unix)]
            Self::Pipe {
                pipe,
                cancel,
                progress,
                worker,
                inputs,
                audio,
                ..
            } => {
                if worker.is_some() {
                    return Ok(());
                }
                let inputs = inputs.take().ok_or_else(|| {
                    MeridianError::Platform("audio mux inputs were already consumed".into())
                })?;
                let pipe_path = pipe.path().to_path_buf();
                let audio = audio.clone();
                let cancel = Arc::clone(cancel);
                let progress = Arc::clone(progress);
                *worker = Some(thread::spawn(move || {
                    let soundfont_cache = SoundfontCache::new();
                    render_audio_pipe_from_cache(
                        inputs.audio_cache.as_ref(),
                        &inputs.audio_config,
                        &soundfont_cache,
                        &audio,
                        RawAudioPipeTarget::Path(pipe_path),
                        cancel.as_ref(),
                        |next| {
                            *progress.lock().expect("audio mux progress mutex poisoned") = next;
                        },
                    )
                }));
                Ok(())
            }
            #[cfg(windows)]
            Self::Pipe {
                pipe,
                cancel,
                progress,
                worker,
                inputs,
                audio,
                ..
            } => {
                if worker.is_some() {
                    return Ok(());
                }
                let inputs = inputs.take().ok_or_else(|| {
                    MeridianError::Platform("audio mux inputs were already consumed".into())
                })?;
                let pipe_file = pipe.connect_writer()?;
                let audio = audio.clone();
                let cancel = Arc::clone(cancel);
                let progress = Arc::clone(progress);
                *worker = Some(thread::spawn(move || {
                    let soundfont_cache = SoundfontCache::new();
                    render_audio_pipe_from_cache(
                        inputs.audio_cache.as_ref(),
                        &inputs.audio_config,
                        &soundfont_cache,
                        &audio,
                        RawAudioPipeTarget::File(pipe_file),
                        cancel.as_ref(),
                        |next| {
                            *progress.lock().expect("audio mux progress mutex poisoned") = next;
                        },
                    )
                }));
                Ok(())
            }
            #[cfg(all(not(unix), not(windows)))]
            Self::Unsupported => Err(MeridianError::Unsupported(
                "muxed audio video export currently requires a supported local pipe implementation"
                    .into(),
            )),
        }
    }

    pub(crate) fn finish(self) -> Result<(), MeridianError> {
        match self {
            #[cfg(any(unix, windows))]
            Self::Pipe { worker, .. } => join_audio_worker(worker),
            #[cfg(all(not(unix), not(windows)))]
            Self::Unsupported => Ok(()),
        }
    }

    pub(crate) fn progress(&self) -> VideoAudioProgress {
        match self {
            #[cfg(any(unix, windows))]
            Self::Pipe { progress, .. } => progress
                .lock()
                .expect("audio mux progress mutex poisoned")
                .clone(),
            #[cfg(all(not(unix), not(windows)))]
            Self::Unsupported => VideoAudioProgress {
                total_events: 0,
                event_index: 0,
                rendered_seconds: 0.0,
            },
        }
    }

    pub(crate) fn cancel_and_join(&mut self) {
        #[cfg(any(unix, windows))]
        {
            let Self::Pipe { cancel, worker, .. } = self;
            cancel.store(true, Ordering::SeqCst);
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }

        #[cfg(all(not(unix), not(windows)))]
        {
            let _ = self;
        }
    }
}

#[cfg(any(unix, windows))]
fn join_audio_worker(
    worker: Option<JoinHandle<Result<(), MeridianError>>>,
) -> Result<(), MeridianError> {
    let Some(worker) = worker else {
        return Ok(());
    };
    worker
        .join()
        .map_err(|_| MeridianError::Platform("audio mux worker panicked".into()))?
}

#[cfg(any(unix, windows))]
fn clip_audio_cache(
    audio_cache: &Arc<InRamAudioCache>,
    time_range: ResolvedVideoTimeRange,
) -> InRamAudioCache {
    let full_length = audio_cache.length();
    if time_range.start_time == 0.0 && (full_length - time_range.end_time).abs() < f64::EPSILON {
        return audio_cache.as_ref().clone();
    }
    audio_cache.clip_time_range(time_range.start_time, time_range.end_time)
}
