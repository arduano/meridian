use super::*;

pub(super) fn wire_midi_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_select_midi_file(move || {
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            let preview_load_generation = Arc::clone(&preview_load_generation);
            let render_load_generation = Arc::clone(&render_load_generation);
            let audio_load_generation = Arc::clone(&audio_load_generation);
            let analysis_load_generation = Arc::clone(&analysis_load_generation);
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let name: slint::SharedString = path.display().to_string().into();
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        replace_selected_midi(
                            &app,
                            &bridge,
                            &shared_state,
                            &preview_load_generation,
                            &render_load_generation,
                            &audio_load_generation,
                            &analysis_load_generation,
                            name,
                        );
                    });
                }
            });
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_drop_midi_file(move |path| {
            if let Some(app) = app_weak.upgrade() {
                if app.get_active_profile() == 6 {
                    append_merge_source_paths(
                        &app,
                        &bridge,
                        &shared_state,
                        vec![PathBuf::from(path.as_str())],
                    );
                    return;
                }
                replace_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                    path.clone(),
                );
                match app.get_active_profile() {
                    0 => load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &path,
                    ),
                    1 | 2 => load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &path,
                    ),
                    3 => load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &path,
                    ),
                    _ => load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &path,
                    ),
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_load_for_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_load_for_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_load_for_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_load_for_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_cancel_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                bridge.cancel_midi_loads();
                preview_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_cancel_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                bridge.cancel_midi_loads();
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_cancel_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                bridge.cancel_midi_loads();
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_analysis_load_state() == MidiLoadState::Loading {
                    app.set_analysis_load_state(MidiLoadState::Selected);
                }
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_cancel_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                bridge.cancel_midi_loads();
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_render(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_retry_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_retry_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_retry_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_retry_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }
}
