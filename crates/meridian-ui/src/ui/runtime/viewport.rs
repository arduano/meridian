use super::*;
use std::time::Instant;

fn approximate_midi_load_fraction(
    path: &Path,
    progress: meridian_core::midi::MidiBuildProgress,
) -> Option<f32> {
    if let Some(fraction_complete) = progress.fraction_complete {
        return Some(fraction_complete);
    }

    let file_bytes = fs::metadata(path).ok()?.len().max(1);

    if let Some(completed_notes) = progress.completed_notes {
        let estimated_total_notes =
            ((file_bytes as f64 / 7.0).round() as u64).max(completed_notes.saturating_add(1));
        return Some((completed_notes as f32 / estimated_total_notes as f32).clamp(0.0, 0.97));
    }

    if let Some(completed_events) = progress.completed_events {
        let estimated_total_events =
            ((file_bytes as f64 / 3.5).round() as u64).max(completed_events.saturating_add(1));
        return Some((completed_events as f32 / estimated_total_events as f32).clamp(0.0, 0.97));
    }

    None
}

/// Install the winit window event handler for OS file drag-and-drop.
pub(super) fn install_drag_drop(app: &App) {
    let app_weak = app.as_weak();
    app.window()
        .on_winit_window_event(move |_slint_window, event| match event {
            winit::event::WindowEvent::HoveredFile(_path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(true);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::DroppedFile(path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    let path_str: slint::SharedString = path.display().to_string().into();
                    app.invoke_drop_midi_file(path_str);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::HoveredFileCancelled => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            _ => EventResult::Propagate,
        });
}

pub(super) fn install_core_event_listener(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    let export_state = Arc::clone(export_state);
    std::thread::spawn(move || {
        for event in receiver {
            update_export_state_from_event(&export_state, &shared_state, &event);
            match &event {
                CoreEvent::MidiLoadProgress {
                    path,
                    progress,
                    status,
                } => {
                    let path_text: slint::SharedString = path.display().to_string().into();
                    let status_text: slint::SharedString = status.clone().into();
                    let progress = approximate_midi_load_fraction(path, *progress);
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        if app.get_selected_midi_name() != path_text {
                            return;
                        }
                        if app.get_render_load_state() == MidiLoadState::Loading {
                            if let Some(progress) = progress {
                                app.set_render_loading_progress(
                                    app.get_render_loading_progress().max(progress),
                                );
                            }
                            app.set_render_loading_status(status_text.clone());
                        }
                        if app.get_audio_load_state() == MidiLoadState::Loading {
                            if let Some(progress) = progress {
                                app.set_audio_loading_progress(
                                    app.get_audio_loading_progress().max(progress),
                                );
                            }
                            app.set_audio_loading_status(status_text);
                        }
                        app.window().request_redraw();
                    });
                }
                CoreEvent::MidiAnalysisJob {
                    event:
                        meridian_core::protocol::MidiAnalysisJobEvent::Progress {
                            job_id,
                            progress,
                            status,
                        },
                } => {
                    let job_id = *job_id;
                    let progress = *progress;
                    let status_text: slint::SharedString = status.clone().into();
                    let shared_state = Arc::clone(&shared_state);
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        let model = shared_state.lock().expect("shared UI state mutex poisoned");
                        if app.get_analysis_load_state() != MidiLoadState::Loading {
                            return;
                        }
                        if model.analysis.pending_job_id != Some(job_id) {
                            return;
                        }
                        app.set_analysis_loading_progress(
                            app.get_analysis_loading_progress().max(progress),
                        );
                        app.set_analysis_loading_status(status_text);
                        app.window().request_redraw();
                    });
                }
                CoreEvent::MidiAnalysisJobStatus { status } => {
                    let status = status.clone();
                    let shared_state = Arc::clone(&shared_state);
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        if !analysis_status_matches_pending(&app, &shared_state, &status) {
                            return;
                        }
                        let event = CoreEvent::MidiAnalysisJobStatus {
                            status: status.clone(),
                        };
                        reduce_core_events(&shared_state, std::slice::from_ref(&event));
                        apply_events_to_app(&app, &shared_state, &[event]);
                        update_analysis_status_ui(&app, &status);
                        app.window().request_redraw();
                    });
                }
                CoreEvent::AudioRender { .. }
                | CoreEvent::AudioRenderStatus { .. }
                | CoreEvent::VideoRender { .. }
                | CoreEvent::VideoRenderStatus { .. }
                | CoreEvent::Error { .. } => {
                    let event = event.clone();
                    let shared_state = Arc::clone(&shared_state);
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        reduce_core_events(&shared_state, std::slice::from_ref(&event));
                        apply_events_to_app(&app, &shared_state, &[event]);
                        app.window().request_redraw();
                    });
                }
                _ => {}
            }
        }
    });
}

fn analysis_status_matches_pending(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    status: &meridian_core::protocol::MidiAnalysisJobStatus,
) -> bool {
    if app.get_analysis_load_state() != MidiLoadState::Loading {
        return false;
    }
    let model = shared_state.lock().expect("shared UI state mutex poisoned");
    let (job_id, parsed_midi_id) = match status {
        meridian_core::protocol::MidiAnalysisJobStatus::Running {
            job_id,
            parsed_midi_id,
            ..
        }
        | meridian_core::protocol::MidiAnalysisJobStatus::Finished {
            job_id,
            parsed_midi_id,
            ..
        }
        | meridian_core::protocol::MidiAnalysisJobStatus::Failed {
            job_id,
            parsed_midi_id,
            ..
        } => (*job_id, *parsed_midi_id),
    };
    model.analysis.pending_job_id == Some(job_id)
        || (model.analysis.pending_job_id.is_none()
            && model.analysis.pending_parsed_midi_id == Some(parsed_midi_id))
}

fn update_analysis_status_ui(app: &App, status: &meridian_core::protocol::MidiAnalysisJobStatus) {
    match status {
        meridian_core::protocol::MidiAnalysisJobStatus::Running {
            progress, status, ..
        } => {
            app.set_analysis_loading_progress(app.get_analysis_loading_progress().max(*progress));
            app.set_analysis_loading_status(status.clone().into());
        }
        meridian_core::protocol::MidiAnalysisJobStatus::Finished { .. } => {
            app.set_analysis_load_state(MidiLoadState::Loaded);
            app.set_analysis_loading_progress(1.0);
            app.set_analysis_loading_status("Analysis ready".into());
            app.set_analysis_load_error(Default::default());
        }
        meridian_core::protocol::MidiAnalysisJobStatus::Failed { message, .. } => {
            app.set_analysis_load_state(MidiLoadState::Error);
            app.set_analysis_load_error(message.clone().into());
            app.set_analysis_loading_status(Default::default());
        }
    }
}

fn update_export_state_from_event(
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
    shared_state: &Arc<Mutex<UiViewModel>>,
    event: &CoreEvent,
) {
    let mut export = export_state
        .lock()
        .expect("render export coordinator mutex poisoned");
    if !export.active {
        return;
    }

    let Some(mode) = export.mode else {
        return;
    };
    let Some(final_output) = export.final_output.clone() else {
        return;
    };

    match (mode, event) {
        (
            RenderExportMode::VideoAudio | RenderExportMode::VideoOnly,
            CoreEvent::VideoRender { event },
        ) => match event {
            meridian_core::protocol::VideoRenderEvent::RenderFinished { output, .. }
                if output == &final_output =>
            {
                // Wait for the subsequent status reset before declaring success so the UI
                // doesn't race ahead of the final render job teardown.
            }
            meridian_core::protocol::VideoRenderEvent::RenderCancelled { .. } => {
                export.outcome = Some(RenderJobOutcome::Cancelled);
            }
            meridian_core::protocol::VideoRenderEvent::RenderFailed { message } => {
                export.outcome = Some(RenderJobOutcome::Failed(message.clone()));
            }
            _ => {}
        },
        (RenderExportMode::AudioOnly, CoreEvent::AudioRender { event }) => match event {
            meridian_core::audio::AudioRenderEvent::RenderFinished { output, .. }
                if output == &final_output =>
            {
                // Wait for the status transition back to idle before finishing in the UI.
            }
            meridian_core::audio::AudioRenderEvent::RenderCancelled { .. } => {
                export.outcome = Some(RenderJobOutcome::Cancelled);
            }
            meridian_core::audio::AudioRenderEvent::RenderFailed { message } => {
                export.outcome = Some(RenderJobOutcome::Failed(message.clone()));
            }
            _ => {}
        },
        (
            RenderExportMode::VideoAudio | RenderExportMode::VideoOnly,
            CoreEvent::VideoRenderStatus {
                status: meridian_core::protocol::VideoRenderStatus::Idle,
            },
        ) if export.outcome.is_none() && video_render_was_active(shared_state) => {
            export.outcome = Some(RenderJobOutcome::Finished);
        }
        (
            RenderExportMode::AudioOnly,
            CoreEvent::AudioRenderStatus {
                status: meridian_core::protocol::AudioRenderStatus::Idle,
            },
        ) if export.outcome.is_none() && audio_render_was_active(shared_state) => {
            export.outcome = Some(RenderJobOutcome::Finished);
        }
        _ => {}
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

fn runtime_state_poll_interval(playing: bool) -> Duration {
    if playing {
        Duration::from_millis(16)
    } else {
        Duration::from_millis(250)
    }
}

fn update_render_export_ui(app: &App, export_state: &Arc<Mutex<RenderExportCoordinator>>) {
    enum UiAction {
        None,
        Finished(PathBuf),
        Failed(String),
        Cancelled,
    }

    let action = {
        let mut export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return;
        }

        let Some(mode) = export.mode else {
            return;
        };

        let outcome = export.outcome.clone();
        match outcome {
            Some(RenderJobOutcome::Failed(message)) => {
                export.active = false;
                UiAction::Failed(message)
            }
            Some(RenderJobOutcome::Cancelled) => {
                export.active = false;
                UiAction::Cancelled
            }
            Some(RenderJobOutcome::Finished) => {
                export.active = false;
                UiAction::Finished(
                    export
                        .final_output
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("render.out")),
                )
            }
            None => {
                let progress = match mode {
                    RenderExportMode::VideoAudio => app
                        .get_video_render_progress()
                        .min(app.get_audio_render_progress()),
                    RenderExportMode::VideoOnly => app.get_video_render_progress(),
                    RenderExportMode::AudioOnly => app.get_audio_render_progress(),
                };
                let status = match mode {
                    RenderExportMode::VideoAudio => "Rendering video + audio",
                    RenderExportMode::VideoOnly => "Rendering video",
                    RenderExportMode::AudioOnly => "Rendering audio",
                };
                set_export_status(
                    app,
                    status,
                    export
                        .final_output
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "working".into()),
                    progress,
                );
                UiAction::None
            }
        }
    };

    match action {
        UiAction::None => {}
        UiAction::Finished(output) => {
            set_export_status(app, "Finished", output.display().to_string(), 1.0);
            if app.get_render_open_after_export() {
                let _ = open::that_detached(&output);
            }
        }
        UiAction::Failed(message) => {
            set_export_status(app, "Failed", message, 0.0);
        }
        UiAction::Cancelled => {
            set_export_status(app, "Cancelled", "Render cancelled".to_string(), 0.0);
        }
    }
}

pub(super) fn install_viewport(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> Result<(), MeridianError> {
    if disable_wgpu {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
        return Ok(());
    }

    let renderer = std::rc::Rc::new(std::cell::RefCell::new(ViewportRenderer::new(
        app.as_weak(),
        core.clone(),
        Arc::clone(shared_state),
        Rc::clone(pending_viewport_image),
        Rc::clone(viewport_size),
        true,
    )));
    let renderer_for_notifier = std::rc::Rc::clone(&renderer);
    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            renderer_for_notifier
                .borrow_mut()
                .handle(state, graphics_api);
        })
        .map_err(|e| MeridianError::SlintNotifier(e.to_string()))
}

pub(super) fn install_timer(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) -> slint::Timer {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let bridge_for_timer = bridge.clone();
    let shared_state_for_timer = Arc::clone(shared_state);
    let pending_viewport_image_for_timer = Rc::clone(pending_viewport_image);
    let viewport_size_for_timer = Rc::clone(viewport_size);
    let export_state_for_timer = Arc::clone(export_state);
    let last_state_refresh = Rc::new(RefCell::new(None::<Instant>));
    let last_state_refresh_for_timer = Rc::clone(&last_state_refresh);

    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(app) = app_for_timer.upgrade() {
                let has_viewport = matches!(app.get_active_profile(), 0 | 1 | 3);
                *viewport_size_for_timer.borrow_mut() = (
                    app.get_viewport_px_width().max(1.0) as u32,
                    app.get_viewport_px_height().max(1.0) as u32,
                );
                if has_viewport {
                    if let Some(image) = pending_viewport_image_for_timer.borrow_mut().take() {
                        app.set_viewport_image(image);
                    }
                }
                if has_viewport && !app_is_loading(&app) {
                    let playing = shared_state_for_timer
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .transport
                        .playing;
                    let now = Instant::now();
                    let should_refresh = {
                        let mut last_refresh = last_state_refresh_for_timer.borrow_mut();
                        let should_refresh = last_refresh
                            .map(|last| {
                                now.duration_since(last) >= runtime_state_poll_interval(playing)
                            })
                            .unwrap_or(true);
                        if should_refresh {
                            *last_refresh = Some(now);
                        }
                        should_refresh
                    };

                    if should_refresh {
                        if let Ok(events) = bridge_for_timer.refresh_state(&shared_state_for_timer)
                        {
                            apply_events_to_app(&app, &shared_state_for_timer, &events);
                            let playing = shared_state_for_timer
                                .lock()
                                .expect("shared UI state mutex poisoned")
                                .transport
                                .playing;
                            if playing || disable_wgpu {
                                app.window().request_redraw();
                            }
                        }
                    }
                }
                if has_viewport && (disable_wgpu || app.get_play_label() == "Pause") {
                    app.window().request_redraw();
                }
                update_render_export_ui(&app, &export_state_for_timer);
            }
        },
    );
    animation_timer
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::{
        audio::AudioRenderEvent,
        protocol::{
            AudioRenderJobId, AudioRenderStatus, CoreEvent, VideoOutputContainer, VideoRenderEvent,
            VideoRenderJobId, VideoRenderStatus,
        },
    };

    fn export_state(mode: RenderExportMode, output: &str) -> Arc<Mutex<RenderExportCoordinator>> {
        Arc::new(Mutex::new(RenderExportCoordinator {
            active: true,
            mode: Some(mode),
            final_output: Some(PathBuf::from(output)),
            outcome: None,
        }))
    }

    fn shared_state() -> Arc<Mutex<UiViewModel>> {
        Arc::new(Mutex::new(UiViewModel::default()))
    }

    #[test]
    fn video_audio_waits_for_idle_status_before_finishing() {
        let export_state = export_state(RenderExportMode::VideoAudio, "render.mp4");
        let shared_state = shared_state();
        shared_state
            .lock()
            .expect("shared UI state mutex poisoned")
            .render_jobs
            .video = VideoRenderStatus::Running {
            job_id: VideoRenderJobId(1),
            output: PathBuf::from("render.mp4"),
            container: VideoOutputContainer::Mp4,
            fps: 30.0,
            width: 1280,
            height: 720,
            total_frames: 10,
            frame_index: 10,
            current_time: 0.0,
            elapsed_seconds: 1.0,
            audio_progress: None,
        };

        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::VideoRender {
                event: VideoRenderEvent::RenderFinished {
                    job_id: VideoRenderJobId(1),
                    total_frames: 10,
                    elapsed_seconds: 1.0,
                    average_fps: 10.0,
                    output: PathBuf::from("render.mp4"),
                    container: VideoOutputContainer::Mp4,
                    exports: Default::default(),
                },
            },
        );
        assert!(
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .outcome
                .is_none()
        );

        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::VideoRenderStatus {
                status: VideoRenderStatus::Idle,
            },
        );
        assert!(matches!(
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .outcome,
            Some(RenderJobOutcome::Finished)
        ));
    }

    #[test]
    fn cancelled_export_does_not_flip_to_finished_on_idle_status() {
        let export_state = export_state(RenderExportMode::VideoOnly, "render.mp4");
        let shared_state = shared_state();
        shared_state
            .lock()
            .expect("shared UI state mutex poisoned")
            .render_jobs
            .video = VideoRenderStatus::Cancelling {
            job_id: VideoRenderJobId(1),
            output: PathBuf::from("render.mp4"),
            container: VideoOutputContainer::Mp4,
            fps: 30.0,
            width: 1280,
            height: 720,
            total_frames: 10,
            frame_index: 3,
            current_time: 0.0,
            elapsed_seconds: 0.3,
            audio_progress: None,
        };

        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::VideoRender {
                event: VideoRenderEvent::RenderCancelled {
                    job_id: VideoRenderJobId(1),
                    frame_index: 3,
                    total_frames: 10,
                    elapsed_seconds: 0.3,
                },
            },
        );
        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::VideoRenderStatus {
                status: VideoRenderStatus::Idle,
            },
        );

        assert!(matches!(
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .outcome,
            Some(RenderJobOutcome::Cancelled)
        ));
    }

    #[test]
    fn audio_only_waits_for_idle_status_before_finishing() {
        let export_state = export_state(RenderExportMode::AudioOnly, "render.wav");
        let shared_state = shared_state();
        shared_state
            .lock()
            .expect("shared UI state mutex poisoned")
            .render_jobs
            .audio = AudioRenderStatus::Running {
            job_id: AudioRenderJobId(1),
            output: PathBuf::from("render.wav"),
            total_events: 10,
            event_index: 10,
            time_seconds: 1.0,
            rendered_seconds: 1.0,
            frames_written: 48_000,
        };

        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::AudioRender {
                event: AudioRenderEvent::RenderFinished {
                    job_id: AudioRenderJobId(1),
                    output: PathBuf::from("render.wav"),
                    frames_written: 48_000,
                    rendered_seconds: 1.0,
                },
            },
        );
        assert!(
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .outcome
                .is_none()
        );

        update_export_state_from_event(
            &export_state,
            &shared_state,
            &CoreEvent::AudioRenderStatus {
                status: AudioRenderStatus::Idle,
            },
        );

        assert!(matches!(
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .outcome,
            Some(RenderJobOutcome::Finished)
        ));
    }

    #[test]
    fn runtime_state_poll_interval_slows_down_while_idle() {
        assert_eq!(runtime_state_poll_interval(true), Duration::from_millis(16));
        assert_eq!(
            runtime_state_poll_interval(false),
            Duration::from_millis(250)
        );
    }
}
