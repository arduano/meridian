use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use crate::{
    audio::{
        AudioBackend, AudioRenderConfig, AudioRenderEvent, MeridianSoundfont, SoundfontCache,
        render_audio_from_cache,
    },
    protocol::{AudioRenderJobId, AudioRenderStatus, CoreErrorCode, CoreEvent},
};

use super::{
    core_state::{AudioRenderJobState, CoreState},
    support::error_event,
};

impl CoreState {
    pub(super) fn request_cancel_render_audio(&mut self) -> bool {
        let Some(job) = &mut self.audio_render_job else {
            return false;
        };

        job.cancel.store(true, Ordering::SeqCst);
        if let AudioRenderStatus::Running {
            job_id,
            output,
            total_events,
            event_index,
            time_seconds,
            rendered_seconds,
            frames_written,
        } = &job.status
        {
            job.status = AudioRenderStatus::Cancelling {
                job_id: *job_id,
                output: output.clone(),
                total_events: *total_events,
                event_index: *event_index,
                time_seconds: *time_seconds,
                rendered_seconds: *rendered_seconds,
                frames_written: *frames_written,
            };
        }

        true
    }

    pub(super) fn active_audio_render_job_id_from_state(&self) -> Option<AudioRenderJobId> {
        self.audio_render_job.as_ref().and_then(|job| match &job.status {
            AudioRenderStatus::Running { job_id, .. }
            | AudioRenderStatus::Cancelling { job_id, .. } => Some(*job_id),
            AudioRenderStatus::Idle => None,
        })
    }

    pub(super) fn start_render_audio(&mut self, config: AudioRenderConfig) -> Vec<CoreEvent> {
        if self.audio_render_job.is_some() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "an audio render is already active",
            )];
        }

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
        let mut audio_config = config
            .audio
            .clone()
            .unwrap_or_else(|| self.audio_config.clone());
        if matches!(audio_config.backend, AudioBackend::None) {
            audio_config.backend = AudioBackend::Xsynth;
        }
        if !config.soundfonts.is_empty() {
            audio_config.soundfonts = config
                .soundfonts
                .iter()
                .cloned()
                .map(|path| MeridianSoundfont {
                    path: Some(path),
                    ..MeridianSoundfont::default()
                })
                .collect();
        }
        let soundfont_cache = SoundfontCache::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let job_id = AudioRenderJobId(self.next_resource_id);
        self.next_resource_id += 1;
        self.active_audio_render_job_id = Some(job_id);
        let initial_status = AudioRenderStatus::Running {
            job_id,
            output: config.output.clone(),
            total_events: 0,
            event_index: 0,
            time_seconds: 0.0,
            rendered_seconds: 0.0,
            frames_written: 0,
        };

        let core_handle = self.core_handle.clone();
        let worker_cancel = Arc::clone(&cancel);
        let worker = thread::spawn(move || {
            let _ = render_audio_from_cache(
                audio_cache.as_ref(),
                &audio_config,
                &soundfont_cache,
                &config,
                job_id,
                &worker_cancel,
                |event| {
                    let _ = core_handle.publish_audio_event(event);
                },
            );
        });
        self.audio_render_job = Some(AudioRenderJobState {
            cancel: Arc::clone(&cancel),
            worker: Some(worker),
            status: initial_status,
        });

        vec![CoreEvent::AudioRenderStatus {
            status: self.audio_render_status(),
        }]
    }

    pub(super) fn cancel_render_audio(&mut self) -> Vec<CoreEvent> {
        if self.request_cancel_render_audio() {
            vec![CoreEvent::AudioRenderStatus {
                status: self.audio_render_status(),
            }]
        } else {
            vec![error_event(
                CoreErrorCode::InvalidCommand,
                "no active audio render",
            )]
        }
    }

    pub(super) fn handle_audio_render_update(&mut self, event: AudioRenderEvent) {
        match &event {
            AudioRenderEvent::RenderStarted {
                job_id,
                output,
                total_events,
                ..
            } => {
                self.audio_render_job =
                    self.audio_render_job.take().map(|job| AudioRenderJobState {
                        cancel: job.cancel,
                        worker: job.worker,
                        status: AudioRenderStatus::Running {
                            job_id: *job_id,
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
                job_id,
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
                            job_id: *job_id,
                            output,
                            total_events: *total_events,
                            event_index: *event_index,
                            time_seconds: *time_seconds,
                            rendered_seconds: *rendered_seconds,
                            frames_written: *frames_written,
                        },
                        AudioRenderStatus::Cancelling { .. } => AudioRenderStatus::Cancelling {
                            job_id: *job_id,
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
                if let Some(mut job) = self.audio_render_job.take() {
                    if let Some(worker) = job.worker.take() {
                        let _ = worker.join();
                    }
                }
                self.active_audio_render_job_id = None;
            }
        }

        self.broadcast(CoreEvent::AudioRender { event });
        self.broadcast(CoreEvent::AudioRenderStatus {
            status: self.audio_render_status(),
        });
    }

    pub(super) fn audio_render_status(&self) -> AudioRenderStatus {
        self.audio_render_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(AudioRenderStatus::Idle)
    }
}
