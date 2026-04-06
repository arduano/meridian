use super::*;

pub(in super::super) fn wire_render_export_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) {
    {
        let app_weak = app.as_weak();
        app.on_select_render_mode(move |mode| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = match mode.as_str() {
                "video_only" => "video_only",
                "audio_only" => "audio_only",
                _ => "video_audio",
            };
            app.set_render_mode_text(mode.into());
            sync_render_output_path(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_resolution(move |preset| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_resolution_text(preset.clone());
            if let Ok((width, height)) = parse_render_resolution(preset.as_str()) {
                app.set_render_video_width_text(width.to_string().into());
                app.set_render_video_height_text(height.to_string().into());
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_resolution(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            // Accept "WxH" or "W×H" or "W H"
            let normalized = val.replace('×', "x").replace(' ', "x");
            if parse_render_resolution(&normalized).is_ok() {
                app.set_render_video_resolution_text(normalized.into());
                if let Ok((w, h)) =
                    parse_render_resolution(app.get_render_video_resolution_text().as_str())
                {
                    app.set_render_video_width_text(w.to_string().into());
                    app.set_render_video_height_text(h.to_string().into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_fps(move |fps| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_fps_text(fps);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_fps(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if parse_render_fps(val.as_str()).is_ok() {
                app.set_render_video_fps_text(val);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_change_render_video_ffmpeg_args(move |args| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_ffmpeg_args_text(args);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_change_render_audio_ffmpeg_args(move |args| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_ffmpeg_args_text(args);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_codec(move |codec| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_codec_text(codec);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_crf(move |crf| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_crf_text(crf);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_video_crf(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            // Validate CRF is a number 0-51
            if let Ok(n) = val.trim().parse::<u32>() {
                if n <= 51 {
                    app.set_render_video_crf_text(n.to_string().into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_preset(move |preset| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_preset_text(preset);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_pix_fmt(move |fmt| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_pix_fmt_text(fmt);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_bitrate(move |rate| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_bitrate_text(rate);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_audio_bitrate(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                app.set_render_audio_bitrate_text(trimmed.into());
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_toggle_render_use_limiter(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_use_limiter(!app.get_render_use_limiter());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_toggle_render_open_after_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_open_after_export(!app.get_render_open_after_export());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_format(move |format| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let format = match format.as_str() {
                "flac" => "flac",
                "mp3" => "mp3",
                _ => "wav",
            };
            app.set_render_audio_format_text(format.into());
            sync_render_output_path(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_sample_rate(move |rate| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_sample_rate_text(rate);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_channel_count(move |channels| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_channel_count_text(channels);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_browse_render_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
            let audio_format =
                AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
            let suggested_output = current_export_output_path(&app).unwrap_or_else(|_| {
                PathBuf::from(default_render_output_path(
                    app.get_selected_midi_name().as_str(),
                    mode,
                    audio_format,
                ))
            });
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new();
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                dialog = match mode {
                    RenderExportMode::VideoAudio | RenderExportMode::VideoOnly => {
                        dialog.add_filter("MP4", &["mp4"])
                    }
                    RenderExportMode::AudioOnly => match audio_format {
                        AudioOnlyFormat::Wav => dialog.add_filter("WAV", &["wav"]),
                        AudioOnlyFormat::Flac => dialog.add_filter("FLAC", &["flac"]),
                        AudioOnlyFormat::Mp3 => dialog.add_filter("MP3", &["mp3"]),
                    },
                };
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_output_path(&path, mode, audio_format);
                    app.set_render_output_path_text(normalized.display().to_string().into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let export_state = Arc::clone(export_state);
        app.on_start_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
            let final_output = match current_export_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_export_status(&app, "Failed", message, 0.0);
                    app.window().request_redraw();
                    return;
                }
            };

            {
                let mut export = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                if export.active {
                    return;
                }
                *export = RenderExportCoordinator {
                    active: true,
                    mode: Some(mode),
                    final_output: Some(final_output.clone()),
                    ..RenderExportCoordinator::default()
                };
            }

            app.set_render_output_path_text(final_output.display().to_string().into());

            if mode.wants_audio() {
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    fail_export(
                        &app,
                        &export_state,
                        "audio cache is already loading; wait for it to finish first".into(),
                    );
                    return;
                }
                if app.get_audio_load_state() != MidiLoadState::Loaded {
                    let midi_path = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.midi_path.clone())
                        .or_else(|| {
                            let selected = app.get_selected_midi_name();
                            (!selected.is_empty()).then(|| PathBuf::from(selected.as_str()))
                        });
                    let Some(midi_path) = midi_path else {
                        fail_export(&app, &export_state, "load a MIDI before exporting".into());
                        return;
                    };

                    app.set_audio_load_state(MidiLoadState::Loading);
                    app.set_audio_loading_progress(0.0);
                    app.set_audio_loading_status("Preparing audio cache…".into());
                    app.set_audio_load_error(Default::default());
                    set_export_status(
                        &app,
                        "Preparing audio cache",
                        final_output.display().to_string(),
                        0.0,
                    );
                    app.window().request_redraw();

                    let app_weak = app.as_weak();
                    let bridge = bridge.clone();
                    let shared_state = Arc::clone(&shared_state);
                    let export_state = Arc::clone(&export_state);
                    std::thread::spawn(move || {
                        let result = bridge.load_audio_midi(midi_path, &shared_state);
                        let _ = app_weak.upgrade_in_event_loop(move |app| match result {
                            Ok(events) => {
                                if let Some(message) = events_error_message(&events) {
                                    app.set_audio_load_state(MidiLoadState::Error);
                                    app.set_audio_load_error(message.clone().into());
                                    app.set_audio_loading_status(Default::default());
                                    fail_export(&app, &export_state, message);
                                    return;
                                }
                                apply_events_to_app(&app, &shared_state, &events);
                                app.set_audio_load_state(MidiLoadState::Loaded);
                                app.set_audio_loading_progress(1.0);
                                app.set_audio_loading_status("Audio ready".into());
                                if export_state
                                    .lock()
                                    .expect("render export coordinator mutex poisoned")
                                    .active
                                {
                                    if let Err(message) = start_render_export_jobs(
                                        &app,
                                        &bridge,
                                        &shared_state,
                                        &export_state,
                                    ) {
                                        fail_export(&app, &export_state, message);
                                    }
                                }
                            }
                            Err(error) => {
                                app.set_audio_load_state(MidiLoadState::Error);
                                app.set_audio_load_error(error.to_string().into());
                                app.set_audio_loading_status(Default::default());
                                fail_export(&app, &export_state, error.to_string());
                            }
                        });
                    });
                    return;
                }
            }

            set_export_status(
                &app,
                "Preparing export",
                final_output.display().to_string(),
                0.0,
            );
            if let Err(message) =
                start_render_export_jobs(&app, &bridge, &shared_state, &export_state)
            {
                fail_export(&app, &export_state, message);
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let export_state = Arc::clone(export_state);
        app.on_cancel_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let (active, finalizing) = {
                let export = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                (export.active, export.finalizing)
            };
            if !active {
                return;
            }

            if finalizing {
                set_export_status(
                    &app,
                    "Finalizing output",
                    "Cancel is unavailable during final mux/encode".to_string(),
                    app.get_render_export_progress(),
                );
                app.window().request_redraw();
                return;
            }

            let video_running = app.get_video_render_status() != "Idle";
            let audio_running = app.get_audio_render_status() != "Idle";
            if video_running {
                if let Ok(events) = bridge.cancel_render_video(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
            }
            if audio_running {
                if let Ok(events) = bridge.cancel_render_audio(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
            }

            if !video_running && !audio_running {
                clear_export_state(&export_state);
                set_export_status(&app, "Cancelled", "Render cancelled".to_string(), 0.0);
            }
            app.window().request_redraw();
        });
    }
}
