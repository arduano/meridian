use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use crate::{
    protocol::{CoreErrorCode, CoreEvent, VideoRenderConfig, VideoRenderEvent, VideoRenderStatus},
    video::render_video,
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

        let cancel = Arc::new(AtomicBool::new(false));
        self.render_job = Some(RenderJobState {
            cancel: Arc::clone(&cancel),
            status: VideoRenderStatus::Running {
                output: config.output.clone(),
                fps: config.fps,
                width: config.width,
                height: config.height,
                total_frames: 0,
                frame_index: 0,
                current_time: 0.0,
                elapsed_seconds: 0.0,
            },
        });

        let core_handle = self.core_handle.clone();
        thread::spawn(move || {
            let _ = render_video(&core_handle, &config, &cancel, |event| {
                let _ = core_handle.publish_video_event(event);
            });
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
                    output,
                    fps,
                    width,
                    height,
                    total_frames,
                    frame_index,
                    current_time,
                    elapsed_seconds,
                } = &job.status
                {
                    job.status = VideoRenderStatus::Cancelling {
                        output: output.clone(),
                        fps: *fps,
                        width: *width,
                        height: *height,
                        total_frames: *total_frames,
                        frame_index: *frame_index,
                        current_time: *current_time,
                        elapsed_seconds: *elapsed_seconds,
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
                output,
                fps,
                width,
                height,
                total_frames,
                ..
            } => {
                self.render_job = self.render_job.take().map(|job| RenderJobState {
                    cancel: job.cancel,
                    status: VideoRenderStatus::Running {
                        output: output.clone(),
                        fps: *fps,
                        width: *width,
                        height: *height,
                        total_frames: *total_frames,
                        frame_index: 0,
                        current_time: 0.0,
                        elapsed_seconds: 0.0,
                    },
                });
            }
            VideoRenderEvent::RenderProgress {
                frame_index,
                total_frames,
                current_time,
                elapsed_seconds,
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
                            output,
                            fps,
                            width,
                            height,
                            total_frames: *total_frames,
                            frame_index: *frame_index,
                            current_time: *current_time,
                            elapsed_seconds: *elapsed_seconds,
                        },
                        VideoRenderStatus::Cancelling { .. } => VideoRenderStatus::Cancelling {
                            output,
                            fps,
                            width,
                            height,
                            total_frames: *total_frames,
                            frame_index: *frame_index,
                            current_time: *current_time,
                            elapsed_seconds: *elapsed_seconds,
                        },
                        VideoRenderStatus::Idle => return,
                    };
                }
            }
            VideoRenderEvent::RenderCancelled { .. }
            | VideoRenderEvent::RenderFinished { .. }
            | VideoRenderEvent::RenderFailed { .. } => {
                self.render_job = None;
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
