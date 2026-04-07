use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use crate::{
    protocol::{
        CoreErrorCode, CoreEvent, VideoRenderConfig, VideoRenderEvent, VideoRenderJobId,
        VideoRenderStatus,
    },
    video::{VideoRenderAudioInputs, render_video},
};

use super::{
    core_state::{CoreState, RenderJobState},
    support::error_event,
};

impl CoreState {
    pub(super) fn start_render_video(&mut self, config: VideoRenderConfig) -> Vec<CoreEvent> {
        if self.render_job.is_some() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "a video render is already active",
            )];
        }

        let audio_inputs = if config.audio.is_some() {
            if let Some(path) = config.midi_path.clone() {
                let current_path = self.snapshot().midi_path;
                if current_path.as_ref() != Some(&path) || self.current_audio_cache.is_none() {
                    let events = self.load_audio_midi(path);
                    if !events
                        .iter()
                        .all(|event| matches!(event, CoreEvent::MidiLoaded { .. }))
                    {
                        return events;
                    }
                }
            }
            let Some(audio_cache) = self.current_audio_cache.clone() else {
                return vec![error_event(
                    CoreErrorCode::NoMidiLoaded,
                    "no midi loaded for audio render",
                )];
            };
            Some(VideoRenderAudioInputs {
                audio_cache,
                audio_config: self.audio_config.clone(),
            })
        } else {
            None
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let job_id = VideoRenderJobId(self.next_resource_id);
        self.next_resource_id += 1;
        self.active_video_render_job_id = Some(job_id);
        self.render_job = Some(RenderJobState {
            cancel: Arc::clone(&cancel),
            status: VideoRenderStatus::Running {
                job_id,
                output: config.output.clone(),
                fps: config.fps,
                width: config.width,
                height: config.height,
                total_frames: 0,
                frame_index: 0,
                current_time: 0.0,
                elapsed_seconds: 0.0,
                audio_progress: None,
            },
        });

        let core_handle = self.core_handle.clone();
        thread::spawn(move || {
            let _ = render_video(
                job_id,
                &core_handle,
                &config,
                audio_inputs,
                &cancel,
                |event| {
                    let _ = core_handle.publish_video_event(event);
                },
            );
        });

        vec![CoreEvent::VideoRenderStatus {
            status: self.video_render_status(),
        }]
    }

    pub(super) fn cancel_render_video(&mut self) -> Vec<CoreEvent> {
        match &mut self.render_job {
            Some(job) => {
                job.cancel.store(true, Ordering::SeqCst);
                if let VideoRenderStatus::Running {
                    job_id,
                    output,
                    fps,
                    width,
                    height,
                    total_frames,
                    frame_index,
                    current_time,
                    elapsed_seconds,
                    audio_progress,
                } = &job.status
                {
                    job.status = VideoRenderStatus::Cancelling {
                        job_id: *job_id,
                        output: output.clone(),
                        fps: *fps,
                        width: *width,
                        height: *height,
                        total_frames: *total_frames,
                        frame_index: *frame_index,
                        current_time: *current_time,
                        elapsed_seconds: *elapsed_seconds,
                        audio_progress: audio_progress.clone(),
                    };
                }
                vec![CoreEvent::VideoRenderStatus {
                    status: self.video_render_status(),
                }]
            }
            None => vec![error_event(
                CoreErrorCode::InvalidCommand,
                "no active video render",
            )],
        }
    }

    pub(super) fn handle_video_render_update(&mut self, event: VideoRenderEvent) {
        match &event {
            VideoRenderEvent::RenderStarted {
                job_id,
                output,
                fps,
                width,
                height,
                total_frames,
                audio_progress,
                ..
            } => {
                self.render_job = self.render_job.take().map(|job| RenderJobState {
                    cancel: job.cancel,
                    status: VideoRenderStatus::Running {
                        job_id: *job_id,
                        output: output.clone(),
                        fps: *fps,
                        width: *width,
                        height: *height,
                        total_frames: *total_frames,
                        frame_index: 0,
                        current_time: 0.0,
                        elapsed_seconds: 0.0,
                        audio_progress: audio_progress.clone(),
                    },
                });
            }
            VideoRenderEvent::RenderProgress {
                job_id,
                frame_index,
                total_frames,
                current_time,
                elapsed_seconds,
                audio_progress,
                ..
            } => {
                if let Some(job) = &mut self.render_job {
                    let (output, fps, width, height) = match &job.status {
                        VideoRenderStatus::Running {
                            output,
                            fps,
                            width,
                            height,
                            ..
                        }
                        | VideoRenderStatus::Cancelling {
                            output,
                            fps,
                            width,
                            height,
                            ..
                        } => (output.clone(), *fps, *width, *height),
                        VideoRenderStatus::Idle => return,
                    };
                    job.status = match &job.status {
                        VideoRenderStatus::Running { .. } => VideoRenderStatus::Running {
                            job_id: *job_id,
                            output,
                            fps,
                            width,
                            height,
                            total_frames: *total_frames,
                            frame_index: *frame_index,
                            current_time: *current_time,
                            elapsed_seconds: *elapsed_seconds,
                            audio_progress: audio_progress.clone(),
                        },
                        VideoRenderStatus::Cancelling { .. } => VideoRenderStatus::Cancelling {
                            job_id: *job_id,
                            output,
                            fps,
                            width,
                            height,
                            total_frames: *total_frames,
                            frame_index: *frame_index,
                            current_time: *current_time,
                            elapsed_seconds: *elapsed_seconds,
                            audio_progress: audio_progress.clone(),
                        },
                        VideoRenderStatus::Idle => return,
                    };
                }
            }
            VideoRenderEvent::RenderCancelled { .. }
            | VideoRenderEvent::RenderFinished { .. }
            | VideoRenderEvent::RenderFailed { .. } => {
                self.render_job = None;
                self.active_video_render_job_id = None;
            }
        }

        self.broadcast(CoreEvent::VideoRender { event });
        self.broadcast(CoreEvent::VideoRenderStatus {
            status: self.video_render_status(),
        });
    }

    pub(super) fn video_render_status(&self) -> VideoRenderStatus {
        self.render_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(VideoRenderStatus::Idle)
    }
}
