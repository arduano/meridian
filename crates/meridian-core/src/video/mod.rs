mod ffmpeg;

use std::{
    io::Write,
    path::PathBuf,
    process::{Child, ChildStdin},
    sync::atomic::{AtomicBool, Ordering},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};

use crate::{
    CoreHandle, MeridianError,
    audio::{
        AudioConfig, SoundfontCache, render_audio_pipe_from_cache, resolve_video_audio_settings,
    },
    midi::audio_cache::InRamAudioCache,
    protocol::{
        CoreCommand, CoreEvent, VideoAudioProgress, VideoExportArtifacts, VideoRenderConfig,
        VideoRenderEvent, VideoRenderJobId,
    },
    render::{
        export::ExportFrame,
        headless::{HeadlessClearMode, HeadlessRenderSession},
    },
    spawn_core,
};

pub use ffmpeg::{VideoFfmpegAudioInput, spawn_ffmpeg_gray, spawn_ffmpeg_rgba};

pub struct VideoRenderAudioInputs {
    pub audio_cache: Arc<InRamAudioCache>,
    pub audio_config: AudioConfig,
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
        if !self.container.matches_path(&self.output) {
            return Err(MeridianError::InvalidMidi(format!(
                "video output path must use .{} for the selected container",
                self.container.extension()
            )));
        }
        Ok(())
    }
}

pub fn render_video(
    job_id: VideoRenderJobId,
    core: &CoreHandle,
    config: &VideoRenderConfig,
    audio_inputs: Option<VideoRenderAudioInputs>,
    cancel: &Arc<AtomicBool>,
    mut on_event: impl FnMut(VideoRenderEvent),
) -> Result<(), MeridianError> {
    let original_state = read_core_state(core)?;
    let result = render_video_inner(job_id, core, config, audio_inputs, cancel, &mut on_event);
    let restore_result = restore_core_state(core, &original_state);

    if let Err(error) = result {
        on_event(VideoRenderEvent::RenderFailed {
            message: error.to_string(),
        });
        return Err(error);
    }
    if let Err(error) = restore_result {
        on_event(VideoRenderEvent::RenderFailed {
            message: error.to_string(),
        });
        return Err(error);
    }
    Ok(())
}

fn render_video_inner(
    job_id: VideoRenderJobId,
    core: &CoreHandle,
    config: &VideoRenderConfig,
    audio_inputs: Option<VideoRenderAudioInputs>,
    cancel: &Arc<AtomicBool>,
    on_event: &mut impl FnMut(VideoRenderEvent),
) -> Result<(), MeridianError> {
    config.validate()?;

    if should_use_isolated_core(config) {
        let isolated = spawn_core();
        let result =
            render_video_with_core(job_id, &isolated, config, audio_inputs, cancel, on_event);
        let _ = isolated.request(CoreCommand::Shutdown);
        return result;
    }

    render_video_with_core(job_id, core, config, audio_inputs, cancel, on_event)
}

fn should_use_isolated_core(config: &VideoRenderConfig) -> bool {
    config.midi_path.is_some()
        && config.scene.is_some()
        && config.view_range.is_some()
        && config.first_key.is_some()
        && config.last_key.is_some()
}

fn render_video_with_core(
    job_id: VideoRenderJobId,
    core: &CoreHandle,
    config: &VideoRenderConfig,
    audio_inputs: Option<VideoRenderAudioInputs>,
    cancel: &Arc<AtomicBool>,
    on_event: &mut impl FnMut(VideoRenderEvent),
) -> Result<(), MeridianError> {
    let default_layout = crate::render::SceneLayout::default();
    let default_view_range = default_layout.view_range;
    let default_first_key = default_layout.first_key;
    let default_last_key = default_layout.last_key;

    let state = read_core_state(core)?;

    if let Some(scene) = &config.scene {
        core.request(CoreCommand::SetSceneConfig {
            scene: scene.clone(),
        })?;
    }
    core.request(CoreCommand::SetViewRange {
        seconds: config
            .view_range
            .unwrap_or(if should_use_isolated_core(config) {
                default_view_range
            } else {
                state.view_range
            }),
        time_space: config.time_space,
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: config
            .first_key
            .unwrap_or(if should_use_isolated_core(config) {
                default_first_key
            } else {
                state.first_key
            }),
        last_key: config
            .last_key
            .unwrap_or(if should_use_isolated_core(config) {
                default_last_key
            } else {
                state.last_key
            }),
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
    let mut audio_mux = match config.audio.as_ref() {
        Some(audio) => Some(VideoAudioMux::spawn(config, audio, audio_inputs, cancel)?),
        None => None,
    };
    let (mut ffmpeg_child, mut ffmpeg_stdin, ffmpeg_command) = spawn_ffmpeg_rgba(
        &config.output,
        config.container,
        config.fps,
        config.width,
        config.height,
        &config.ffmpeg_args,
        audio_mux
            .as_ref()
            .map(|mux| mux.ffmpeg_input())
            .transpose()?,
    )?;
    if let Some(audio_mux) = audio_mux.as_mut() {
        if let Err(error) = audio_mux.start() {
            drop(ffmpeg_stdin);
            let _ = ffmpeg_child.kill();
            let _ = ffmpeg_child.wait();
            return Err(error);
        }
    }
    let alpha_output = config
        .export
        .export_alpha_mask
        .then(|| crate::render::export::sidecar_path(&config.output, "alpha"));
    let (mut alpha_ffmpeg, alpha_ffmpeg_command) =
        match spawn_alpha_ffmpeg(alpha_output.clone(), config) {
            Ok(value) => value,
            Err(error) => {
                drop(ffmpeg_stdin);
                let _ = ffmpeg_child.kill();
                let _ = ffmpeg_child.wait();
                if let Some(audio_mux) = audio_mux.as_mut() {
                    audio_mux.cancel_and_join();
                }
                return Err(error);
            }
        };
    let mut session = HeadlessRenderSession::new_with_clear_mode(
        &frame0.layout,
        config.width,
        config.height,
        HeadlessClearMode::Transparent,
    )?;

    on_event(VideoRenderEvent::RenderStarted {
        job_id,
        midi: config.midi_path.clone(),
        output: config.output.clone(),
        container: config.container,
        exports: VideoExportArtifacts {
            alpha_mask: alpha_output.clone(),
        },
        fps: config.fps,
        width: config.width,
        height: config.height,
        total_frames,
        duration_seconds,
        ffmpeg_command,
        alpha_ffmpeg_command,
        audio_progress: audio_mux.as_ref().map(|mux| mux.progress()),
    });

    let start = Instant::now();
    core.request(CoreCommand::ResetProjectorPhysics)?;
    let mut color_frame = Vec::new();
    let mut alpha_frame = Vec::new();
    for frame_index in 0..total_frames {
        if cancel.load(Ordering::SeqCst) {
            drop(ffmpeg_stdin);
            let _ = ffmpeg_child.kill();
            let _ = ffmpeg_child.wait();
            if let Some(encoder) = alpha_ffmpeg.as_mut() {
                drop(encoder.stdin.take());
                let _ = encoder.child.kill();
                let _ = encoder.child.wait();
            }
            if let Some(audio_mux) = audio_mux.as_mut() {
                audio_mux.cancel_and_join();
            }
            on_event(VideoRenderEvent::RenderCancelled {
                job_id,
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
        let export_frame = ExportFrame::new(config.width, config.height, rgba);
        export_frame.fill_color_rgba(config.export.color_mode, &mut color_frame);
        ffmpeg_stdin.write_all(&color_frame)?;
        if let Some(encoder) = alpha_ffmpeg.as_mut() {
            export_frame.fill_alpha_luma(&mut alpha_frame);
            if let Some(stdin) = encoder.stdin.as_mut() {
                stdin.write_all(&alpha_frame)?;
            }
        }

        let elapsed_seconds = start.elapsed().as_secs_f64();
        let average_fps = if elapsed_seconds > 0.0 {
            (frame_index + 1) as f64 / elapsed_seconds
        } else {
            0.0
        };
        on_event(VideoRenderEvent::RenderProgress {
            job_id,
            frame_index: frame_index + 1,
            total_frames,
            current_time,
            elapsed_seconds,
            average_fps,
            audio_progress: audio_mux.as_ref().map(|mux| mux.progress()),
        });
    }

    drop(ffmpeg_stdin);
    if let Some(encoder) = alpha_ffmpeg.as_mut() {
        let _ = encoder.stdin.take();
    }
    let status = ffmpeg_child.wait()?;
    if !status.success() {
        if let Some(audio_mux) = audio_mux.as_mut() {
            audio_mux.cancel_and_join();
        }
        return Err(MeridianError::Platform(format!(
            "ffmpeg exited with status {status}"
        )));
    }
    if let Err(error) = wait_for_alpha_ffmpeg(alpha_ffmpeg) {
        if let Some(audio_mux) = audio_mux.as_mut() {
            audio_mux.cancel_and_join();
        }
        return Err(error);
    }
    if let Some(audio_mux) = audio_mux.take() {
        audio_mux.finish()?;
    }

    let elapsed_seconds = start.elapsed().as_secs_f64();
    let average_fps = if elapsed_seconds > 0.0 {
        total_frames as f64 / elapsed_seconds
    } else {
        0.0
    };
    on_event(VideoRenderEvent::RenderFinished {
        job_id,
        total_frames,
        elapsed_seconds,
        average_fps,
        output: config.output.clone(),
        container: config.container,
        exports: VideoExportArtifacts {
            alpha_mask: alpha_output,
        },
    });
    Ok(())
}

enum VideoAudioMux {
    #[cfg(unix)]
    Pipe {
        fifo: crate::ffmpeg::FifoGuard,
        sample_rate: u32,
        channels: u16,
        cancel: Arc<AtomicBool>,
        progress: Arc<Mutex<VideoAudioProgress>>,
        worker: Option<JoinHandle<Result<(), MeridianError>>>,
        inputs: Option<VideoRenderAudioInputs>,
        audio: crate::protocol::VideoAudioConfig,
    },
}

impl VideoAudioMux {
    fn spawn(
        config: &VideoRenderConfig,
        audio: &crate::protocol::VideoAudioConfig,
        audio_inputs: Option<VideoRenderAudioInputs>,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Self, MeridianError> {
        #[cfg(unix)]
        {
            let inputs = audio_inputs.ok_or_else(|| {
                MeridianError::Platform("missing cached audio data for muxed video render".into())
            })?;
            let (sample_rate, channels, _) =
                resolve_video_audio_settings(&inputs.audio_config, audio)?;
            let total_events = inputs.audio_cache.events().len();
            let fifo = crate::ffmpeg::FifoGuard::create(&config.output, "audio")?;
            let _ = cancel;
            return Ok(Self::Pipe {
                fifo,
                sample_rate,
                channels,
                cancel: Arc::clone(cancel),
                progress: Arc::new(Mutex::new(VideoAudioProgress {
                    total_events,
                    event_index: 0,
                    rendered_seconds: 0.0,
                })),
                worker: None,
                inputs: Some(inputs),
                audio: audio.clone(),
            });
        }

        #[cfg(not(unix))]
        {
            let _ = (config, audio, audio_inputs, cancel);
            Err(MeridianError::Unsupported(
                "muxed audio video export currently requires unix named pipes".into(),
            ))
        }
    }

    fn ffmpeg_input(&self) -> Result<VideoFfmpegAudioInput<'_>, MeridianError> {
        match self {
            #[cfg(unix)]
            Self::Pipe {
                fifo,
                sample_rate,
                channels,
                audio,
                ..
            } => Ok(VideoFfmpegAudioInput {
                pipe_path: fifo.path(),
                sample_rate: *sample_rate,
                channels: *channels,
                extra_args: &audio.ffmpeg_args,
            }),
        }
    }

    fn start(&mut self) -> Result<(), MeridianError> {
        match self {
            #[cfg(unix)]
            Self::Pipe {
                fifo,
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
                let pipe_path = fifo.path().to_path_buf();
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
                        &pipe_path,
                        cancel.as_ref(),
                        |next| {
                            *progress.lock().expect("audio mux progress mutex poisoned") = next;
                        },
                    )
                }));
                Ok(())
            }
        }
    }

    fn finish(self) -> Result<(), MeridianError> {
        match self {
            #[cfg(unix)]
            Self::Pipe { worker, .. } => join_audio_worker(worker),
        }
    }

    fn progress(&self) -> VideoAudioProgress {
        match self {
            #[cfg(unix)]
            Self::Pipe { progress, .. } => progress
                .lock()
                .expect("audio mux progress mutex poisoned")
                .clone(),
        }
    }

    fn cancel_and_join(&mut self) {
        #[cfg(unix)]
        let Self::Pipe { cancel, worker, .. } = self;
        {
            cancel.store(true, Ordering::SeqCst);
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

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

struct AlphaFfmpeg {
    child: Child,
    stdin: Option<ChildStdin>,
}

fn spawn_alpha_ffmpeg(
    alpha_output: Option<PathBuf>,
    config: &VideoRenderConfig,
) -> Result<(Option<AlphaFfmpeg>, Option<Vec<String>>), MeridianError> {
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
        Some(AlphaFfmpeg {
            child,
            stdin: Some(stdin),
        }),
        Some(command),
    ))
}

fn wait_for_alpha_ffmpeg(alpha_ffmpeg: Option<AlphaFfmpeg>) -> Result<(), MeridianError> {
    let Some(mut alpha_ffmpeg) = alpha_ffmpeg else {
        return Ok(());
    };
    let status = alpha_ffmpeg.child.wait()?;
    if !status.success() {
        return Err(MeridianError::Platform(format!(
            "alpha ffmpeg exited with status {status}"
        )));
    }
    Ok(())
}

fn read_core_state(core: &CoreHandle) -> Result<crate::protocol::StateSnapshot, MeridianError> {
    match core.request(CoreCommand::GetState)? {
        events => match events.into_iter().next() {
            Some(CoreEvent::StateSnapshot { state }) => Ok(state),
            _ => Err(MeridianError::Protocol(
                "unexpected response while reading core state".into(),
            )),
        },
    }
}

fn restore_core_state(
    core: &CoreHandle,
    state: &crate::protocol::StateSnapshot,
) -> Result<(), MeridianError> {
    if let Some(path) = &state.midi_path {
        core.request(CoreCommand::LoadMidi { path: path.clone() })?;
    } else {
        core.request(CoreCommand::UnloadRenderContext)?;
    }
    core.request(CoreCommand::SetSceneConfig {
        scene: state.scene.clone(),
    })?;
    core.request(CoreCommand::SetViewRange {
        seconds: state.view_range,
        time_space: Some(state.time_space),
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: state.first_key,
        last_key: state.last_key,
    })?;
    core.request(CoreCommand::SetViewport {
        width: state.viewport_width,
        height: state.viewport_height,
    })?;
    core.request(CoreCommand::SetTime {
        time: state.current_time,
    })?;
    core.request(CoreCommand::SetPlaying {
        playing: state.playing,
    })?;
    Ok(())
}
