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
    export_state: &Arc<Mutex<RenderExportRuntime>>,
) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    let export_state = Arc::clone(export_state);
    std::thread::spawn(move || {
        for event in receiver {
            export_state
                .lock()
                .expect("render export coordinator mutex poisoned")
                .observe_core_event(&shared_state, &event);
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

fn runtime_state_poll_interval(playing: bool) -> Duration {
    if playing {
        Duration::from_millis(16)
    } else {
        Duration::from_millis(250)
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
        app.set_status_text("Accelerated viewport disabled via --disable-wgpu".into());
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
    export_state: &Arc<Mutex<RenderExportRuntime>>,
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
                if has_viewport
                    && let Some(image) = pending_viewport_image_for_timer.borrow_mut().take() {
                        app.set_viewport_image(image);
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

                    if should_refresh
                        && let Ok(events) = bridge_for_timer.refresh_state(&shared_state_for_timer)
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
                if has_viewport && (disable_wgpu || app.get_play_label() == "Pause") {
                    app.window().request_redraw();
                }
                let snapshot = export_state_for_timer
                    .lock()
                    .expect("render export coordinator mutex poisoned")
                    .snapshot(RenderExportProgress::from_app(&app));
                if let Some(snapshot) = snapshot {
                    let RenderExportUiSnapshot {
                        status,
                        detail,
                        progress,
                        terminal,
                    } = snapshot;
                    set_export_status(&app, status, detail, progress);
                    if let Some(RenderExportUiTerminal::Finished(output)) = terminal
                        && app.get_render_open_after_export() {
                            let _ = open::that_detached(&output);
                        }
                }
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

    fn export_state(mode: RenderExportMode, output: &str) -> Arc<Mutex<RenderExportRuntime>> {
        let export_state = Arc::new(Mutex::new(RenderExportRuntime::default()));
        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .begin(RenderExportDraft {
                mode,
                audio_format: AudioOnlyFormat::Wav,
                video_container: VideoOutputContainer::Mp4,
                final_output: PathBuf::from(output),
            })
            .expect("render export controller should accept a fresh draft");
        export_state
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

        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
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
        let running_snapshot = export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .snapshot(RenderExportProgress {
                video: 0.8,
                audio: 0.9,
            })
            .expect("running export should still produce a snapshot");
        assert_eq!(running_snapshot.status, "Rendering video + audio");
        assert!(running_snapshot.terminal.is_none());

        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
                &shared_state,
                &CoreEvent::VideoRenderStatus {
                    status: VideoRenderStatus::Idle,
                },
            );
        let finished_snapshot = export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .snapshot(RenderExportProgress::default())
            .expect("finished export should produce a terminal snapshot");
        assert!(matches!(
            finished_snapshot.terminal,
            Some(RenderExportUiTerminal::Finished(output)) if output == *"render.mp4"
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

        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
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
        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
                &shared_state,
                &CoreEvent::VideoRenderStatus {
                    status: VideoRenderStatus::Idle,
                },
            );

        let snapshot = export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .snapshot(RenderExportProgress::default())
            .expect("cancelled export should produce a terminal snapshot");
        assert!(matches!(
            snapshot.terminal,
            Some(RenderExportUiTerminal::Cancelled)
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

        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
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
                .snapshot(RenderExportProgress::default())
                .expect("running export should still produce a snapshot")
                .terminal
                .is_none()
        );

        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .observe_core_event(
                &shared_state,
                &CoreEvent::AudioRenderStatus {
                    status: AudioRenderStatus::Idle,
                },
            );

        let snapshot = export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .snapshot(RenderExportProgress::default())
            .expect("finished export should produce a terminal snapshot");
        assert!(matches!(
            snapshot.terminal,
            Some(RenderExportUiTerminal::Finished(output)) if output == *"render.wav"
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
