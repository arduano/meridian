use super::*;

pub(in super::super) fn initialize_merge_panel(app: &App, shared_state: &Arc<Mutex<UiViewModel>>) {
    apply_merge_sources_to_app(app, shared_state);
    if app.get_merge_output_path_text().is_empty() {
        app.set_merge_result_output_text("merged-output.mid".into());
    }
}

pub(in super::super) fn wire_merge_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_merge_files(move || {
            if app_weak.upgrade().is_none() {
                return;
            }
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
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
                    append_merge_source_paths(&app, &bridge, &shared_state, files);
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
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                model.merge.sources.clear();
            }
            app.set_merge_output_path_text("".into());
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
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                let index = index.max(0) as usize;
                if index < model.merge.sources.len() {
                    model.merge.sources.remove(index);
                }
            }
            apply_merge_sources_to_app(&app, &shared_state);
            if app.get_merge_output_path_text().is_empty() {
                if let Some(path) = merge_default_output_from_model(&shared_state) {
                    app.set_merge_output_path_text(path.display().to_string().into());
                    app.set_merge_result_output_text(file_name_or_path(&path).into());
                }
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
        let bridge = bridge.clone();
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
            app.set_merge_output_path_text(output.display().to_string().into());

            let config = build_merge_config(&app);
            let events = match bridge.merge_midi_files(inputs, output, config, &shared_state) {
                Ok(events) => events,
                Err(error) => {
                    set_merge_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };

            if let Some(message) = events_error_message(&events) {
                set_merge_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_cancel_merge_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            set_merge_ui_failure(&app, "merge does not currently support cancellation");
            app.window().request_redraw();
        });
    }
}

pub(in super::super) fn append_merge_source_paths(
    app: &App,
    bridge: &UiCoreBridge,
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
    app.set_merge_status_text("Inspecting sources".into());
    app.set_merge_detail_text(
        format!(
            "Loading {} new MIDI source{}.",
            pending.len(),
            if pending.len() == 1 { "" } else { "s" }
        )
        .into(),
    );
    apply_merge_sources_to_app(app, shared_state);
    app.window().request_redraw();

    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    std::thread::spawn(move || {
        let result = bridge.inspect_midi_files(pending.clone(), &shared_state);
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                match result {
                    Ok(inspections) => {
                        for inspection in inspections {
                            if let Some(source) = model
                                .merge
                                .sources
                                .iter_mut()
                                .find(|source| source.path == inspection.path)
                            {
                                source.inspection = if let Some(message) = inspection.error.clone()
                                {
                                    MergeSourceInspection::Error(message)
                                } else {
                                    MergeSourceInspection::Ready(inspection)
                                };
                            }
                        }
                    }
                    Err(error) => {
                        for path in pending {
                            if let Some(source) = model
                                .merge
                                .sources
                                .iter_mut()
                                .find(|source| source.path == path)
                            {
                                source.inspection = MergeSourceInspection::Error(error.to_string());
                            }
                        }
                    }
                }
            }
            apply_merge_sources_to_app(&app, &shared_state);
            app.set_merge_status_text("Ready".into());
            app.set_merge_detail_text(
                "Source queue updated. Reorder files, choose a merge recipe, then write output."
                    .into(),
            );
            app.window().request_redraw();
        });
    });
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
