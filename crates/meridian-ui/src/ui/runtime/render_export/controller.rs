use super::*;
use meridian_core::protocol::VideoOutputContainer;

#[derive(Debug, Clone)]
pub(crate) struct RenderExportDraft {
    pub mode: RenderExportMode,
    pub audio_format: AudioOnlyFormat,
    pub video_container: VideoOutputContainer,
    pub final_output: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderExportProgress {
    pub video: f32,
    pub audio: f32,
}

#[derive(Debug, Clone)]
pub(crate) enum RenderExportUiTerminal {
    Finished(PathBuf),
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderExportUiSnapshot {
    pub status: &'static str,
    pub detail: String,
    pub progress: f32,
    pub terminal: Option<RenderExportUiTerminal>,
}

#[derive(Debug, Clone)]
struct RenderExportJob {
    draft: RenderExportDraft,
    outcome: Option<RenderJobOutcome>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RenderExportController {
    job: Option<RenderExportJob>,
}

impl RenderExportDraft {
    pub(crate) fn from_app(app: &App) -> Result<Self, String> {
        let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
        let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
        let video_container =
            video_output_container_from_text(app.get_render_video_container_text().as_str());
        let raw_output = resolve_render_output_path_text(
            app.get_render_output_path_text().as_str(),
            app.get_selected_midi_name().as_str(),
            mode,
            audio_format,
            video_container,
        );
        if raw_output.is_empty() {
            return Err("select a MIDI before exporting".into());
        }
        Ok(Self {
            mode,
            audio_format,
            video_container,
            final_output: PathBuf::from(raw_output),
        })
    }

    pub(crate) fn status_label(&self) -> &'static str {
        match self.mode {
            RenderExportMode::VideoAudio => "Rendering video + audio",
            RenderExportMode::VideoOnly => "Rendering video",
            RenderExportMode::AudioOnly => "Rendering audio",
        }
    }

    pub(crate) fn progress(&self, progress: RenderExportProgress) -> f32 {
        match self.mode {
            RenderExportMode::VideoAudio => progress.video.min(progress.audio),
            RenderExportMode::VideoOnly => progress.video,
            RenderExportMode::AudioOnly => progress.audio,
        }
    }
}

impl RenderExportProgress {
    pub(crate) fn from_app(app: &App) -> Self {
        Self {
            video: app.get_video_render_progress(),
            audio: app.get_audio_render_progress(),
        }
    }
}

impl RenderExportController {
    pub(crate) fn is_active(&self) -> bool {
        self.job.is_some()
    }

    pub(crate) fn begin(&mut self, draft: RenderExportDraft) -> Result<(), String> {
        if self.job.is_some() {
            return Err("export is already active".into());
        }
        self.job = Some(RenderExportJob {
            draft,
            outcome: None,
        });
        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        self.job = None;
    }

    pub(crate) fn draft(&self) -> Option<&RenderExportDraft> {
        self.job.as_ref().map(|job| &job.draft)
    }

    pub(crate) fn observe_core_event(
        &mut self,
        shared_state: &Arc<Mutex<UiViewModel>>,
        event: &CoreEvent,
    ) {
        let Some(job) = self.job.as_mut() else {
            return;
        };
        if job.outcome.is_some() {
            return;
        }

        let final_output = job.draft.final_output.clone();
        match (job.draft.mode, event) {
            (
                RenderExportMode::VideoAudio | RenderExportMode::VideoOnly,
                CoreEvent::VideoRender { event },
            ) => match event {
                meridian_core::protocol::VideoRenderEvent::RenderFinished { output, .. }
                    if output == &final_output =>
                {
                    // Wait for the render status transition back to idle before finishing.
                }
                meridian_core::protocol::VideoRenderEvent::RenderCancelled { .. } => {
                    job.outcome = Some(RenderJobOutcome::Cancelled);
                }
                meridian_core::protocol::VideoRenderEvent::RenderFailed { message } => {
                    job.outcome = Some(RenderJobOutcome::Failed(message.clone()));
                }
                _ => {}
            },
            (RenderExportMode::AudioOnly, CoreEvent::AudioRender { event }) => match event {
                meridian_core::audio::AudioRenderEvent::RenderFinished { output, .. }
                    if output == &final_output =>
                {
                    // Wait for the render status transition back to idle before finishing.
                }
                meridian_core::audio::AudioRenderEvent::RenderCancelled { .. } => {
                    job.outcome = Some(RenderJobOutcome::Cancelled);
                }
                meridian_core::audio::AudioRenderEvent::RenderFailed { message } => {
                    job.outcome = Some(RenderJobOutcome::Failed(message.clone()));
                }
                _ => {}
            },
            (
                RenderExportMode::VideoAudio | RenderExportMode::VideoOnly,
                CoreEvent::VideoRenderStatus {
                    status: meridian_core::protocol::VideoRenderStatus::Idle,
                },
            ) if job.outcome.is_none() && video_render_was_active(shared_state) => {
                job.outcome = Some(RenderJobOutcome::Finished);
            }
            (
                RenderExportMode::AudioOnly,
                CoreEvent::AudioRenderStatus {
                    status: meridian_core::protocol::AudioRenderStatus::Idle,
                },
            ) if job.outcome.is_none() && audio_render_was_active(shared_state) => {
                job.outcome = Some(RenderJobOutcome::Finished);
            }
            _ => {}
        }
    }

    pub(crate) fn snapshot(
        &mut self,
        progress: RenderExportProgress,
    ) -> Option<RenderExportUiSnapshot> {
        let draft = self.draft()?.clone();
        let outcome = self.job.as_ref()?.outcome.clone();
        match outcome {
            Some(RenderJobOutcome::Finished) => {
                let output = draft.final_output.clone();
                self.clear();
                Some(RenderExportUiSnapshot {
                    status: "Finished",
                    detail: output.display().to_string(),
                    progress: 1.0,
                    terminal: Some(RenderExportUiTerminal::Finished(output)),
                })
            }
            Some(RenderJobOutcome::Failed(message)) => {
                self.clear();
                Some(RenderExportUiSnapshot {
                    status: "Failed",
                    detail: message.clone(),
                    progress: 0.0,
                    terminal: Some(RenderExportUiTerminal::Failed),
                })
            }
            Some(RenderJobOutcome::Cancelled) => {
                self.clear();
                Some(RenderExportUiSnapshot {
                    status: "Cancelled",
                    detail: "Render cancelled".to_string(),
                    progress: 0.0,
                    terminal: Some(RenderExportUiTerminal::Cancelled),
                })
            }
            None => Some(RenderExportUiSnapshot {
                status: draft.status_label(),
                detail: draft.final_output.display().to_string(),
                progress: draft.progress(progress),
                terminal: None,
            }),
        }
    }
}

fn video_render_was_active(shared_state: &Arc<Mutex<UiViewModel>>) -> bool {
    matches!(
        shared_state
            .lock()
            .expect("shared UI state mutex poisoned")
            .render_jobs
            .video,
        meridian_core::protocol::VideoRenderStatus::Running { .. }
            | meridian_core::protocol::VideoRenderStatus::Cancelling { .. }
    )
}

fn audio_render_was_active(shared_state: &Arc<Mutex<UiViewModel>>) -> bool {
    matches!(
        shared_state
            .lock()
            .expect("shared UI state mutex poisoned")
            .render_jobs
            .audio,
        meridian_core::protocol::AudioRenderStatus::Running { .. }
            | meridian_core::protocol::AudioRenderStatus::Cancelling { .. }
    )
}
