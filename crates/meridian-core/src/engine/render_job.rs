use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use crate::{
    audio::{AudioRenderConfig, AudioRenderEvent, SoundfontCache, render_audio},
    protocol::{
        AudioRenderStatus, CoreErrorCode, CoreEvent, VideoRenderConfig, VideoRenderEvent,
        VideoRenderStatus,
    },
    video::render_video,
};

use super::{
    core_state::{AudioRenderJobState, CoreState, RenderJobState},
    support::error_event,
};

impl CoreState {
    pub(super) fn start_render_audio(&mut self, config: AudioRenderConfig) -> Vec<CoreEvent> {
        if self.audio_render_job.is_some() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "an audio render is already active",
            )];
        }

        let Some(midi_cache) = self.midi_cache.clone() else {
            return vec![error_event(
                CoreErrorCode::NoMidiLoaded,
                "no midi loaded for audio render",
            )];
        };
        let audio_config = config
            .audio
            .clone()
            .unwrap_or_else(|| self.audio_config.clone());
        let soundfont_cache = SoundfontCache::new();
        let cancel = Arc::new(AtomicBool::new(false));
        self.audio_render_job = Some(AudioRenderJobState {
            cancel: Arc::clone(&cancel),
            status: AudioRenderStatus::Running {
                output: config.output.clone(),
                total_events: 0,
                event_index: 0,
                time_seconds: 0.0,
                rendered_seconds: 0.0,
                frames_written: 0,
            },
        });

        let core_handle = self.core_handle.clone();
        thread::spawn(move || {
            let _ = render_audio(
                &midi_cache,
                &audio_config,
                &soundfont_cache,
                &config,
                &cancel,
                |event| {
                    let _ = core_handle.publish_audio_event(event);
                },
            );
        });

        vec![CoreEvent::AudioRenderStatus {
            status: self.audio_render_status(),
        }]
    }

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

    pub(super) fn cancel_render_audio(&mut self) -> Vec<CoreEvent> {
        match &mut self.audio_render_job {
            Some(job) => {
                job.cancel.store(true, Ordering::SeqCst);
                if let AudioRenderStatus::Running {
                    output,
                    total_events,
                    event_index,
                    time_seconds,
                    rendered_seconds,
                    frames_written,
                } = &job.status
                {
                    job.status = AudioRenderStatus::Cancelling {
                        output: output.clone(),
                        total_events: *total_events,
                        event_index: *event_index,
                        time_seconds: *time_seconds,
                        rendered_seconds: *rendered_seconds,
                        frames_written: *frames_written,
                    };
                }
                vec![CoreEvent::AudioRenderStatus {
                    status: self.audio_render_status(),
                }]
            }
            None => vec![error_event(
                CoreErrorCode::InvalidCommand,
                "no active audio render",
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

    pub(super) fn handle_audio_render_update(&mut self, event: AudioRenderEvent) {
        match &event {
            AudioRenderEvent::RenderStarted {
                output,
                total_events,
                ..
            } => {
                self.audio_render_job =
                    self.audio_render_job.take().map(|job| AudioRenderJobState {
                        cancel: job.cancel,
                        status: AudioRenderStatus::Running {
                            output: output.clone(),
                            total_events: *total_events,
                            event_index: 0,
                            time_seconds: 0.0,
                            rendered_seconds: 0.0,
                            frames_written: 0,
                        },
                    });
            }
            AudioRenderEvent::RenderProgress {
                event_index,
                total_events,
                time_seconds,
                rendered_seconds,
                frames_written,
                ..
            } => {
                if let Some(job) = &mut self.audio_render_job {
                    let output = match &job.status {
                        AudioRenderStatus::Running { output, .. }
                        | AudioRenderStatus::Cancelling { output, .. } => output.clone(),
                        AudioRenderStatus::Idle => return,
                    };
                    job.status = match &job.status {
                        AudioRenderStatus::Running { .. } => AudioRenderStatus::Running {
                            output,
                            total_events: *total_events,
                            event_index: *event_index,
                            time_seconds: *time_seconds,
                            rendered_seconds: *rendered_seconds,
                            frames_written: *frames_written,
                        },
                        AudioRenderStatus::Cancelling { .. } => AudioRenderStatus::Cancelling {
                            output,
                            total_events: *total_events,
                            event_index: *event_index,
                            time_seconds: *time_seconds,
                            rendered_seconds: *rendered_seconds,
                            frames_written: *frames_written,
                        },
                        AudioRenderStatus::Idle => return,
                    };
                }
            }
            AudioRenderEvent::RenderFinished { .. }
            | AudioRenderEvent::RenderCancelled { .. }
            | AudioRenderEvent::RenderFailed { .. } => {
                self.audio_render_job = None;
            }
        }

        self.broadcast(CoreEvent::AudioRender { event });
        self.broadcast(CoreEvent::AudioRenderStatus {
            status: self.audio_render_status(),
        });
    }

    pub(super) fn video_render_status(&self) -> VideoRenderStatus {
        self.render_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(VideoRenderStatus::Idle)
    }

    pub(super) fn audio_render_status(&self) -> AudioRenderStatus {
        self.audio_render_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(AudioRenderStatus::Idle)
    }
}
