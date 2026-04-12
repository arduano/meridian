use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

static MERGE_SOURCE_INSPECTION_GENERATION: AtomicU64 = AtomicU64::new(0);

fn invalidate_merge_source_inspection() {
    MERGE_SOURCE_INSPECTION_GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn next_merge_source_inspection_generation() -> u64 {
    MERGE_SOURCE_INSPECTION_GENERATION
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1)
}

fn merge_source_inspection_is_current(generation: u64) -> bool {
    MERGE_SOURCE_INSPECTION_GENERATION.load(Ordering::SeqCst) == generation
}

fn loading_merge_sources_from_model(shared_state: &Arc<Mutex<UiViewModel>>) -> Vec<PathBuf> {
    shared_state
        .lock()
        .expect("ui model mutex poisoned")
        .merge
        .sources
        .iter()
        .filter(|source| matches!(source.inspection, MergeSourceInspection::Loading))
        .map(|source| source.path.clone())
        .collect()
}

fn refresh_merge_source_inspection(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending: Vec<PathBuf>,
) {
    if pending.is_empty() {
        return;
    }

    let generation = next_merge_source_inspection_generation();
    app.set_merge_status_text("Inspecting sources".into());
    app.set_merge_detail_text(
        format!(
            "Loading {} MIDI source{}.",
            pending.len(),
            if pending.len() == 1 { "" } else { "s" }
        )
        .into(),
    );
    app.set_merge_progress(0.02);
    apply_merge_sources_to_app(app, shared_state);
    app.window().request_redraw();

    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    std::thread::spawn(move || {
        let total = pending.len().max(1);
        for (index, path) in pending.iter().cloned().enumerate() {
            let inspection = meridian_core::midi::inspect::inspect_midi_file(path);
            let shared_state = Arc::clone(&shared_state);
            let progress = ((index + 1) as f32 / total as f32).clamp(0.0, 1.0);
            let detail = format!(
                "Inspected {} of {} source{}.",
                index + 1,
                total,
                if total == 1 { "" } else { "s" }
            );
            let _ = app_weak.upgrade_in_event_loop(move |app| {
                if !merge_source_inspection_is_current(generation) {
                    return;
                }

                let mut updated = false;
                {
                    let mut model = shared_state.lock().expect("ui model mutex poisoned");
                    if let Some(source) = model
                        .merge
                        .sources
                        .iter_mut()
                        .find(|source| source.path == inspection.path)
                    {
                        source.inspection = if let Some(message) = inspection.error.clone() {
                            MergeSourceInspection::Error(message)
                        } else {
                            MergeSourceInspection::Ready(inspection)
                        };
                        updated = true;
                    }
                }
                if !updated {
                    return;
                }

                apply_merge_sources_to_app(&app, &shared_state);
                app.set_merge_progress(progress);
                if progress < 1.0 {
                    app.set_merge_status_text("Inspecting sources".into());
                    app.set_merge_detail_text(detail.into());
                } else {
                    app.set_merge_status_text("Ready".into());
                    app.set_merge_detail_text(
                        "Source queue updated. Reorder files, choose a merge recipe, then write output."
                            .into(),
                    );
                    app.set_merge_progress(0.0);
                }
                app.window().request_redraw();
            });
        }
    });
}

pub(in super::super) fn initialize_merge_panel(app: &App, shared_state: &Arc<Mutex<UiViewModel>>) {
    apply_merge_sources_to_app(app, shared_state);
    if app.get_merge_output_path_text().is_empty() {
        app.set_merge_result_output_text("merged-output.mid".into());
    }
}

pub(in super::super) fn wire_merge_callbacks(
    app: &App,
    _bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_merge_files(move || {
            if app_weak.upgrade().is_none() {
                return;
            }
            let app_weak = app_weak.clone();
            let shared_state = Arc::clone(&shared_state);
            std::thread::spawn(move || {
                let files = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_files()
                    .unwrap_or_default();
                if files.is_empty() {
                    return;
                }
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    append_merge_source_paths(&app, &shared_state, files);
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_clear_merge_files(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            invalidate_merge_source_inspection();
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                model.merge.sources.clear();
            }
            app.set_merge_output_path_text("".into());
            app.set_merge_progress(0.0);
            app.set_merge_status_text("Ready".into());
            app.set_merge_detail_text("Drop MIDI files here or browse to assemble a merge.".into());
            app.set_merge_result_output_text("merged-output.mid".into());
            apply_merge_sources_to_app(&app, &shared_state);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_remove_merge_source(move |index| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            let mut removed = false;
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                let index = index.max(0) as usize;
                if index < model.merge.sources.len() {
                    model.merge.sources.remove(index);
                    removed = true;
                }
            }
            if !removed {
                return;
            }
            invalidate_merge_source_inspection();
            apply_merge_sources_to_app(&app, &shared_state);
            if app.get_merge_output_path_text().is_empty() {
                if let Some(path) = merge_default_output_from_model(&shared_state) {
                    app.set_merge_output_path_text(path.display().to_string().into());
                    app.set_merge_result_output_text(file_name_or_path(&path).into());
                }
            }
            let pending = loading_merge_sources_from_model(&shared_state);
            if pending.is_empty() {
                app.set_merge_progress(0.0);
                app.set_merge_status_text("Ready".into());
                app.set_merge_detail_text(if merge_inputs_from_model(&shared_state).is_empty() {
                    "Drop MIDI files here or browse to assemble a merge.".into()
                } else {
                    "Source queue updated. Reorder files, choose a merge recipe, then write output."
                        .into()
                });
            } else {
                refresh_merge_source_inspection(&app, &shared_state, pending);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_move_merge_source(move |index, delta| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                let index = index.max(0) as usize;
                if index >= model.merge.sources.len() {
                    return;
                }
                let target = index as isize + delta as isize;
                if !(0..model.merge.sources.len() as isize).contains(&target) {
                    return;
                }
                model.merge.sources.swap(index, target as usize);
            }
            apply_merge_sources_to_app(&app, &shared_state);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_update_merge_control(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_merge_control(&app, key.as_str(), value.as_str());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_browse_merge_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let suggested_output = current_merge_output_path(&app)
                .ok()
                .or_else(|| merge_default_output_from_model(&shared_state))
                .unwrap_or_else(|| PathBuf::from("merged-output.mid"));
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new().add_filter("MIDI", &["mid", "midi"]);
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_merge_output_path(&path);
                    app.set_merge_output_path_text(normalized.display().to_string().into());
                    app.set_merge_result_output_text(file_name_or_path(&normalized).into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_start_merge_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }

            let inputs = merge_inputs_from_model(&shared_state);
            if inputs.is_empty() {
                set_merge_ui_failure(&app, "add at least one MIDI file");
                app.window().request_redraw();
                return;
            }

            let output = match current_merge_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_merge_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };
            if let Err(message) = validate_merge_job_output_path(&inputs, &output) {
                set_merge_ui_failure(&app, &message);
                app.window().request_redraw();
                return;
            }
            app.set_merge_output_path_text(output.display().to_string().into());

            let config = build_merge_config(&app);
            let cancel = Arc::new(AtomicBool::new(false));
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                model.merge.cancel = Some(Arc::clone(&cancel));
            }
            app.set_merge_job_active(true);
            app.set_merge_progress(0.0);
            app.set_merge_status_text("Processing MIDI".into());
            app.set_merge_detail_text(format!("Writing {}", output.display()).into());
            app.set_merge_result_output_text(file_name_or_path(&output).into());
            app.window().request_redraw();

            let app_weak = app_weak.clone();
            let shared_state = Arc::clone(&shared_state);
            let output_label = file_name_or_path(&output);
            let cancel_for_worker = Arc::clone(&cancel);
            std::thread::spawn(move || {
                let app_weak_for_progress = app_weak.clone();
                let output_label_for_progress = output_label.clone();
                let cancel_for_progress = Arc::clone(&cancel_for_worker);
                let merge_result =
                    meridian_core::midi::file_merge::merge_midi_files_to_file_with_progress_cancelable(
                        &inputs,
                        &output,
                        &config,
                        move || cancel_for_worker.load(Ordering::SeqCst),
                        move |progress| {
                            let fraction = (progress.progress_percent / 100.0).clamp(0.0, 1.0);
                            let detail = progress.status;
                            let output_label = output_label_for_progress.clone();
                            let cancel_for_ui = Arc::clone(&cancel_for_progress);
                            let _ = app_weak_for_progress.upgrade_in_event_loop(move |app| {
                                app.set_merge_progress(fraction);
                                app.set_merge_status_text(
                                    if cancel_for_ui.load(Ordering::SeqCst) {
                                        "Cancelling".into()
                                    } else {
                                        "Processing MIDI".into()
                                    },
                                );
                                app.set_merge_detail_text(detail.into());
                                app.set_merge_result_output_text(output_label.into());
                                app.window().request_redraw();
                            });
                        },
                    );
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    shared_state
                        .lock()
                        .expect("ui model mutex poisoned")
                        .merge
                        .cancel = None;
                    match merge_result {
                        Ok(summary) => {
                            app.set_merge_job_active(false);
                            app.set_merge_progress(1.0);
                            apply_events_to_app(
                                &app,
                                &shared_state,
                                &[CoreEvent::MidiFilesMerged {
                                    output: summary.output,
                                    input_count: summary.input_count,
                                    output_track_count: summary.output_track_count,
                                    output_ppq: summary.output_ppq,
                                    total_events: summary.total_events,
                                }],
                            );
                        }
                        Err(MeridianError::Cancelled(_)) => {
                            app.set_merge_job_active(false);
                            app.set_merge_progress(0.0);
                            app.set_merge_status_text("Cancelled".into());
                            app.set_merge_detail_text("Merge cancelled".into());
                            app.set_merge_result_output_text(output_label.into());
                        }
                        Err(error) => {
                            set_merge_ui_failure(&app, &error.to_string());
                        }
                    }
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_cancel_merge_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let cancel = shared_state
                .lock()
                .expect("ui model mutex poisoned")
                .merge
                .cancel
                .clone();
            let Some(cancel) = cancel else {
                return;
            };
            cancel.store(true, Ordering::SeqCst);
            app.set_merge_status_text("Cancelling".into());
            app.set_merge_detail_text("Stopping merge after the current step.".into());
            app.window().request_redraw();
        });
    }
}

pub(in super::super) fn append_merge_source_paths(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    paths: Vec<PathBuf>,
) {
    let mut pending = Vec::new();
    {
        let mut model = shared_state.lock().expect("ui model mutex poisoned");
        for path in paths {
            if model.merge.sources.iter().any(|source| source.path == path) {
                continue;
            }
            model
                .merge
                .sources
                .push(MergeSourceViewModel::new_loading(path.clone()));
            pending.push(path);
        }
    }

    if pending.is_empty() {
        return;
    }

    if app.get_merge_output_path_text().is_empty() {
        if let Some(path) = merge_default_output_from_model(shared_state) {
            app.set_merge_output_path_text(path.display().to_string().into());
            app.set_merge_result_output_text(file_name_or_path(&path).into());
        }
    }
    refresh_merge_source_inspection(
        app,
        shared_state,
        loading_merge_sources_from_model(shared_state),
    );
}

pub(super) fn merge_inputs_from_model(shared_state: &Arc<Mutex<UiViewModel>>) -> Vec<PathBuf> {
    shared_state
        .lock()
        .expect("ui model mutex poisoned")
        .merge
        .sources
        .iter()
        .map(|source| source.path.clone())
        .collect()
}

pub(super) fn merge_default_output_from_model(
    shared_state: &Arc<Mutex<UiViewModel>>,
) -> Option<PathBuf> {
    let inputs = merge_inputs_from_model(shared_state);
    (!inputs.is_empty()).then(|| default_merge_output_path(&inputs))
}
