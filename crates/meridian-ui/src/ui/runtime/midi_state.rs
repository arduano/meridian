use super::*;

pub(super) fn app_is_loading(app: &App) -> bool {
    app_has_active_midi_load(app)
}

pub(super) fn cancel_pending_midi_loads(
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    preview_load_generation.fetch_add(1, Ordering::SeqCst);
    render_load_generation.fetch_add(1, Ordering::SeqCst);
    audio_load_generation.fetch_add(1, Ordering::SeqCst);
    analysis_load_generation.fetch_add(1, Ordering::SeqCst);
}

pub(super) fn reset_midi_ui_state(app: &App, state: MidiLoadState) {
    // Keep the existing viewport texture binding during replacement loads.
    // The renderer can reuse the same off-screen texture, so clearing the
    // bound image here can leave the UI blank until Slint tears it down.
    if state == MidiLoadState::NoMidi {
        app.set_viewport_image(slint::Image::default());
    }
    app.set_render_load_state(state);
    app.set_audio_load_state(state);
    app.set_analysis_load_state(state);
    app.set_audio_load_error(Default::default());
    app.set_render_load_error(Default::default());
    app.set_analysis_load_error(Default::default());
    app.set_audio_loading_progress(0.0);
    app.set_audio_loading_status(Default::default());
    app.set_render_loading_progress(0.0);
    app.set_render_loading_status(Default::default());
    app.set_analysis_loading_progress(0.0);
    app.set_analysis_loading_status(Default::default());
    reset_analysis_outputs(app);
}

pub(super) fn unload_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    bridge.cancel_midi_loads();
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    if let Ok(events) = bridge.drop_inactive_midi_resources(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, Default::default());
    reset_midi_ui_state(app, MidiLoadState::NoMidi);
    app.window().request_redraw();
}

pub(super) fn replace_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
    selected_midi_name: slint::SharedString,
) {
    bridge.cancel_midi_loads();
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    if let Ok(events) = bridge.drop_inactive_midi_resources(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, selected_midi_name);
    reset_midi_ui_state(app, MidiLoadState::Selected);
    app.window().request_redraw();
}

pub(super) fn update_video_scene(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    mutate: impl FnOnce(&mut SceneConfig),
) {
    if let Ok(events) = bridge.update_scene(shared_state, mutate) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

pub(super) fn update_video_key_range(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    first_key: u8,
    last_key: u8,
) {
    if let Ok(events) = bridge.set_key_range(
        first_key.min(last_key),
        first_key.max(last_key),
        shared_state,
    ) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

pub(super) fn update_video_view_range(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    seconds: f64,
) {
    if let Ok(events) =
        bridge.set_view_range_value(seconds.max(MIN_VIEW_RANGE_SECONDS), shared_state)
    {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

pub(super) fn update_audio_config(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    mutate: impl FnOnce(&mut AudioConfig),
) {
    let Some(mut config) = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.audio.clone())
    else {
        return;
    };

    mutate(&mut config);
    if let Ok(events) = bridge.set_audio_config(config, shared_state) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

pub(super) fn primary_soundfont(config: &mut AudioConfig) -> &mut MeridianSoundfont {
    if config.soundfonts.is_empty() {
        config.soundfonts.push(MeridianSoundfont::default());
    }
    config
        .soundfonts
        .first_mut()
        .expect("audio config must always have a primary soundfont")
}

pub(super) fn set_selected_midi(app: &App, selected_midi_name: slint::SharedString) {
    app.set_selected_midi_name(selected_midi_name.clone());
    app.set_window_title(window_title_for_selected(selected_midi_name.as_str()));
    set_default_render_output_path(app);
    if selected_midi_name.is_empty() {
        app.set_modify_output_path_text(Default::default());
    } else {
        app.set_modify_output_path_text(
            default_modify_output_path(selected_midi_name.as_str())
                .display()
                .to_string()
                .into(),
        );
    }
}

pub(super) fn window_title_for_selected(selected_midi_name: &str) -> slint::SharedString {
    let selected_path = PathBuf::from(selected_midi_name);
    let Some(file_name) = selected_path
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
    else {
        return "Meridian".into();
    };

    format!("Meridian - {file_name}").into()
}

pub(super) fn install_midi_process_listener(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    std::thread::spawn(move || {
        for event in receiver {
            let is_modify_event = matches!(
                event,
                CoreEvent::MidiProcess { .. } | CoreEvent::MidiProcessStatus { .. }
            );
            if !is_modify_event {
                continue;
            }

            shared_state
                .lock()
                .expect("shared UI state mutex poisoned")
                .reduce_events(std::slice::from_ref(&event));

            let event_for_ui = event.clone();
            let shared_state = Arc::clone(&shared_state);
            let _ = app_weak.upgrade_in_event_loop(move |app| {
                apply_events_to_app(&app, &shared_state, std::slice::from_ref(&event_for_ui));
                app.window().request_redraw();
            });
        }
    });
}
