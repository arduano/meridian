mod audio_mux;
mod config;
mod core_session;
mod ffmpeg;
mod pipeline;

use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    time::Instant,
};

use crate::{
    CoreHandle, MeridianError,
    protocol::{
        CoreCommand, VideoExportArtifacts, VideoRenderConfig,
        VideoRenderEvent, VideoRenderJobId,
    },
    render::{
        export::ExportFrame,
        headless::{HeadlessClearMode, HeadlessRenderSession},
    },
    spawn_core,
};

pub(crate) use config::should_use_isolated_core;
pub use config::VideoRenderAudioInputs;
pub use ffmpeg::{VideoFfmpegAudioInput, spawn_ffmpeg_gray, spawn_ffmpeg_rgba};
use audio_mux::VideoAudioMux;
use core_session::{read_core_state, restore_core_state};
use pipeline::{spawn_alpha_ffmpeg, VideoRenderPipeline};

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
    let restore_result = match &result {
        Ok(RenderVideoOutcome::Finished { .. }) => restore_core_state(core, &original_state),
        Ok(RenderVideoOutcome::Cancelled { .. }) => Ok(()),
        Err(_) => restore_core_state(core, &original_state),
    };

    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => {
            on_event(VideoRenderEvent::RenderFailed {
                message: error.to_string(),
            });
            return Err(error);
        }
    };

    if let Err(error) = restore_result {
        on_event(VideoRenderEvent::RenderFailed {
            message: error.to_string(),
        });
        return Err(error);
    }

    match outcome {
        RenderVideoOutcome::Finished {
            total_frames,
            elapsed_seconds,
            average_fps,
            alpha_output,
        } => {
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
        }
        RenderVideoOutcome::Cancelled {
            frame_index,
            total_frames,
            elapsed_seconds,
        } => {
            on_event(VideoRenderEvent::RenderCancelled {
                job_id,
                frame_index,
                total_frames,
                elapsed_seconds,
            });
        }
    }

    Ok(())
}

enum RenderVideoOutcome {
    Finished {
        total_frames: u64,
        elapsed_seconds: f64,
        average_fps: f64,
        alpha_output: Option<PathBuf>,
    },
    Cancelled {
        frame_index: u64,
        total_frames: u64,
        elapsed_seconds: f64,
    },
}

fn render_video_inner(
    job_id: VideoRenderJobId,
    core: &CoreHandle,
    config: &VideoRenderConfig,
    audio_inputs: Option<VideoRenderAudioInputs>,
    cancel: &Arc<AtomicBool>,
    on_event: &mut impl FnMut(VideoRenderEvent),
) -> Result<RenderVideoOutcome, MeridianError> {
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

fn render_video_with_core(
    job_id: VideoRenderJobId,
    core: &CoreHandle,
    config: &VideoRenderConfig,
    audio_inputs: Option<VideoRenderAudioInputs>,
    cancel: &Arc<AtomicBool>,
    on_event: &mut impl FnMut(VideoRenderEvent),
) -> Result<RenderVideoOutcome, MeridianError> {
    let default_layout = crate::render::SceneLayout::default();
    let default_view_range = default_layout.view_range;
    let default_first_key = default_layout.first_key;
    let default_last_key = default_layout.last_key;
    let isolated_core = should_use_isolated_core(config);

    let state = read_core_state(core)?;

    if let Some(scene) = &config.scene {
        core.request(CoreCommand::SetSceneConfig {
            scene: scene.clone(),
        })?;
    }
    core.request(CoreCommand::SetViewRange {
        seconds: config
            .view_range
            .unwrap_or(if isolated_core {
                default_view_range
            } else {
                state.view_range
            }),
        time_space: config.time_space,
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: config
            .first_key
            .unwrap_or(if isolated_core {
                default_first_key
            } else {
                state.first_key
            }),
        last_key: config
            .last_key
            .unwrap_or(if isolated_core {
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
    let audio_mux = match config.audio.as_ref() {
        Some(audio) => Some(VideoAudioMux::spawn(config, audio, audio_inputs, cancel)?),
        None => None,
    };
    let (ffmpeg_child, ffmpeg_stdin, ffmpeg_command) = spawn_ffmpeg_rgba(
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
    let mut pipeline = VideoRenderPipeline::new(ffmpeg_child, ffmpeg_stdin, audio_mux);
    pipeline.start_audio_mux()?;
    let alpha_output = config
        .export
        .export_alpha_mask
        .then(|| crate::render::export::sidecar_path(&config.output, "alpha"));
    let (alpha_ffmpeg, alpha_ffmpeg_command) = spawn_alpha_ffmpeg(alpha_output.clone(), config)?;
    pipeline.set_alpha_ffmpeg(alpha_ffmpeg);
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
        audio_progress: pipeline.audio_progress(),
    });

    let start = Instant::now();
    core.request(CoreCommand::ResetProjectorPhysics)?;
    let mut color_frame = Vec::new();
    let mut alpha_frame = Vec::new();
    for frame_index in 0..total_frames {
        if cancel.load(Ordering::SeqCst) {
            pipeline.cancel();
            return Ok(RenderVideoOutcome::Cancelled {
                frame_index,
                total_frames,
                elapsed_seconds: start.elapsed().as_secs_f64(),
            });
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
        pipeline.write_color_frame(&color_frame)?;
        if pipeline.has_alpha_ffmpeg() {
            export_frame.fill_alpha_luma(&mut alpha_frame);
            pipeline.write_alpha_frame(&alpha_frame)?;
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
            audio_progress: pipeline.audio_progress(),
        });
    }

    pipeline.finish()?;

    let elapsed_seconds = start.elapsed().as_secs_f64();
    let average_fps = if elapsed_seconds > 0.0 {
        total_frames as f64 / elapsed_seconds
    } else {
        0.0
    };
    Ok(RenderVideoOutcome::Finished {
        total_frames,
        elapsed_seconds,
        average_fps,
        alpha_output,
    })
}
