use super::*;

pub(super) fn load_preview_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Loading preview playback…".into());
    app.set_audio_loading_status("Loading preview playback…".into());
    app.set_render_loading_progress(0.0);
    app.set_audio_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = preview_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let preview_load_generation = Arc::clone(preview_load_generation);
    std::thread::spawn(move || {
        let result = (|| -> Result<Option<Vec<CoreEvent>>, MeridianError> {
            if !load_request_is_current(&preview_load_generation, request_generation) {
                return Ok(None);
            }
            let mut events = bridge.drop_inactive_midi_resources(&shared_state)?;
            if !load_request_is_current(&preview_load_generation, request_generation) {
                return Ok(None);
            }
            events.extend(bridge.load_midi(path, &shared_state)?);
            if !load_request_is_current(&preview_load_generation, request_generation) {
                return Ok(None);
            }
            events.extend(bridge.set_playing(true, &shared_state)?);
            Ok(Some(events))
        })();
        if !load_request_is_current(&preview_load_generation, request_generation) {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if !load_request_is_current(&preview_load_generation, request_generation) {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(Some(events)) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_audio_loading_progress(1.0);
                    app.set_render_loading_status("Preview ready".into());
                    app.set_audio_loading_status("Preview ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_audio_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                    app.set_audio_loading_status(Default::default());
                }
                Ok(None) => return,
            }
            app.window().request_redraw();
        });
    });
}

/// Start a render-context MIDI load without blocking the Slint event loop.
pub(super) fn load_render_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    render_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Preparing display cache…".into());
    app.set_render_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = render_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let render_load_generation = Arc::clone(render_load_generation);
    std::thread::spawn(move || {
        let result = (|| -> Result<Option<Vec<CoreEvent>>, MeridianError> {
            if !load_request_is_current(&render_load_generation, request_generation) {
                return Ok(None);
            }
            let mut events = bridge.drop_inactive_midi_resources(&shared_state)?;
            if !load_request_is_current(&render_load_generation, request_generation) {
                return Ok(None);
            }
            events.extend(bridge.load_display_midi(path, &shared_state)?);
            Ok(Some(events))
        })();
        if !load_request_is_current(&render_load_generation, request_generation) {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if !load_request_is_current(&render_load_generation, request_generation) {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(Some(events)) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_render_loading_status("Render visuals ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                }
                Ok(None) => return,
            }
            app.window().request_redraw();
        });
    });
}

pub(super) fn load_audio_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    audio_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_audio_loading_status("Preparing audio cache…".into());
    app.set_audio_loading_progress(0.0);
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = audio_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let audio_load_generation = Arc::clone(audio_load_generation);
    std::thread::spawn(move || {
        let result = (|| -> Result<Option<Vec<CoreEvent>>, MeridianError> {
            if !load_request_is_current(&audio_load_generation, request_generation) {
                return Ok(None);
            }
            let mut events = bridge.drop_inactive_midi_resources(&shared_state)?;
            if !load_request_is_current(&audio_load_generation, request_generation) {
                return Ok(None);
            }
            events.extend(bridge.load_audio_midi(path, &shared_state)?);
            Ok(Some(events))
        })();
        if !load_request_is_current(&audio_load_generation, request_generation) {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if !load_request_is_current(&audio_load_generation, request_generation) {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(Some(events)) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_audio_loading_progress(1.0);
                    app.set_audio_loading_status("Audio ready".into());
                }
                Err(e) => {
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_audio_load_error(e.to_string().into());
                    app.set_audio_loading_status(Default::default());
                }
                Ok(None) => return,
            }
            app.window().request_redraw();
        });
    });
}

pub(super) fn load_analysis_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    analysis_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_analysis_load_state(MidiLoadState::Loading);
    app.set_analysis_loading_status("Reading MIDI for analysis…".into());
    app.set_analysis_loading_progress(0.0);
    app.set_analysis_load_error(Default::default());
    reset_analysis_outputs(app);
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = analysis_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    std::thread::spawn(move || {
        let progress = |value: f32, status: &'static str| {
            update_analysis_progress(
                &app_weak,
                &requested_name,
                &analysis_load_generation,
                request_generation,
                value,
                status,
            );
        };
        progress(0.15, "Parsing MIDI for analysis…");
        let result = load_analysis_resource_set(
            &bridge,
            &shared_state,
            &path,
            || load_request_is_current(&analysis_load_generation, request_generation),
            progress,
        );
        if !load_request_is_current(&analysis_load_generation, request_generation) {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if !load_request_is_current(&analysis_load_generation, request_generation) {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(Some(events)) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_analysis_load_state(MidiLoadState::Loaded);
                    app.set_analysis_loading_progress(1.0);
                    app.set_analysis_loading_status("Analysis ready".into());
                }
                Err(e) => {
                    app.set_analysis_load_state(MidiLoadState::Error);
                    app.set_analysis_load_error(e.to_string().into());
                    app.set_analysis_loading_status(Default::default());
                }
                Ok(None) => return,
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_resource_set(
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    path: &std::path::Path,
    is_current: impl Fn() -> bool,
    mut progress: impl FnMut(f32, &'static str),
) -> Result<Option<Vec<CoreEvent>>, MeridianError> {
    if !is_current() {
        return Ok(None);
    }
    let mut all_events = bridge.drop_inactive_midi_resources(shared_state)?;
    if !is_current() {
        return Ok(None);
    }

    let parsed_events = bridge.load_parsed_midi(path.to_path_buf(), shared_state)?;
    let parsed_midi_id = parsed_midi_id_from_events(&parsed_events)?;
    all_events.extend(parsed_events);

    progress(0.55, "Building analysis model…");
    if !is_current() {
        return Ok(None);
    }
    progress(0.82, "Computing bucketed note statistics…");
    if !is_current() {
        return Ok(None);
    }
    let analysis_events = bridge.analyze_parsed_midi(parsed_midi_id, None, shared_state)?;
    all_events.extend(analysis_events);
    if !is_current() {
        return Ok(None);
    }
    all_events.extend(bridge.drop_inactive_midi_resources(shared_state)?);

    Ok(Some(all_events))
}

fn parsed_midi_id_from_events(events: &[CoreEvent]) -> Result<ParsedMidiId, MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing parsed MIDI id".into()))
}

pub(super) fn reset_analysis_outputs(app: &App) {
    app.set_analysis_note_count_text("—".into());
    app.set_analysis_track_count_text("—".into());
    app.set_analysis_midi_length_text("—".into());
    app.set_analysis_tempo_text("—".into());
    app.set_analysis_time_signature_text("—".into());
    app.set_analysis_key_range_text("—".into());
    app.set_analysis_avg_velocity_text("—".into());
    app.set_analysis_note_density_text("—".into());
}

fn update_analysis_progress(
    app_weak: &slint::Weak<App>,
    requested_name: &slint::SharedString,
    analysis_load_generation: &Arc<AtomicU64>,
    request_generation: u64,
    progress: f32,
    status: &str,
) {
    let app_weak = app_weak.clone();
    let requested_name = requested_name.clone();
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    let status_text: slint::SharedString = status.into();
    let _ = app_weak.upgrade_in_event_loop(move |app| {
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        if app.get_selected_midi_name() != requested_name {
            return;
        }
        if app.get_analysis_load_state() != MidiLoadState::Loading {
            return;
        }
        app.set_analysis_loading_progress(progress);
        app.set_analysis_loading_status(status_text);
        app.window().request_redraw();
    });
}

fn load_request_is_current(generation: &Arc<AtomicU64>, request_generation: u64) -> bool {
    generation.load(Ordering::SeqCst) == request_generation
}
