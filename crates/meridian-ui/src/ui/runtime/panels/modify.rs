use super::*;

pub(in super::super) fn initialize_modify_panel(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    load_modify_pass_into_app(app, "quantize");
    if app.get_modify_output_path_text().is_empty() && !app.get_selected_midi_name().is_empty() {
        app.set_modify_output_path_text(
            default_modify_output_path(app.get_selected_midi_name().as_str())
                .display()
                .to_string()
                .into(),
        );
    }
    if let Ok(status) = bridge.get_midi_file_process_status(shared_state) {
        apply_events_to_app(
            app,
            shared_state,
            &[CoreEvent::MidiProcessStatus { status }],
        );
    }
}

pub(in super::super) fn wire_modify_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        app.on_select_modify_pass(move |key| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            load_modify_pass_into_app(&app, key.as_str());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_update_modify_control(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match update_modify_control(&app, key.as_str(), value.as_str()) {
                Ok(()) => app.window().request_redraw(),
                Err(message) => {
                    app.set_modify_config_status_text(
                        format!("Input error: {message}. Config unchanged.").into(),
                    );
                    app.window().request_redraw();
                }
            }
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_edit_modify_config(move |text| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_modify_config_text(text);
            validate_modify_config(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_reset_modify_config(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let current_key = app.get_modify_pass_key_text();
            let key = match current_key.as_str() {
                "" | "custom" => "quantize",
                other => other,
            };
            load_modify_pass_into_app(&app, key);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_browse_modify_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let suggested_output = current_modify_output_path(&app).unwrap_or_else(|_| {
                default_modify_output_path(app.get_selected_midi_name().as_str())
            });
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new().add_filter("MIDI", &["mid", "midi"]);
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_modify_output_path(&path);
                    app.set_modify_output_path_text(normalized.display().to_string().into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_start_modify_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_modify_job_active() {
                return;
            }

            let selected = app.get_selected_midi_name();
            if selected.is_empty() {
                set_modify_ui_failure(&app, "choose a MIDI file first");
                app.window().request_redraw();
                return;
            }

            let output = match current_modify_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_modify_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };
            app.set_modify_output_path_text(output.display().to_string().into());

            let config = match parse_modify_config(&app) {
                Ok(config) => config,
                Err(message) => {
                    set_modify_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };

            let events = match bridge.start_process_midi_file(
                PathBuf::from(selected.as_str()),
                output,
                config,
                &shared_state,
            ) {
                Ok(events) => events,
                Err(error) => {
                    set_modify_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };

            if let Some(message) = events_error_message(&events) {
                set_modify_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_cancel_modify_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let events = match bridge.cancel_midi_file_process(&shared_state) {
                Ok(events) => events,
                Err(error) => {
                    set_modify_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };
            if let Some(message) = events_error_message(&events) {
                set_modify_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
}
