//! Slint callbacks for render/export controls.

use super::*;

pub(in super::super) fn wire_render_export_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportController>>,
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
        app.on_select_render_range_mode(move |mode| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = match mode.as_str() {
                "custom" => "custom",
                _ => "full_song",
            };
            app.set_render_range_mode_text(mode.into());
            if mode == "custom" {
                if app.get_render_start_time_text().is_empty() {
                    apply_default_render_time_range(&app, app.get_midi_length_seconds() as f64);
                } else if app.get_render_end_time_text().is_empty() {
                    let (_, end_time) =
                        default_render_time_range_text(app.get_midi_length_seconds() as f64);
                    app.set_render_end_time_text(end_time.into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_start_time(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if parse_render_time_seconds(val.as_str(), "render start time").is_ok() {
                app.set_render_start_time_text(val.trim().into());
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_end_time(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if parse_render_time_seconds(val.as_str(), "render end time").is_ok() {
                app.set_render_end_time_text(val.trim().into());
            }
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
        app.on_select_render_video_container(move |container| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let container = match container.as_str() {
                "mkv" => "mkv",
                _ => "mp4",
            };
            app.set_render_video_container_text(container.into());
            sync_render_output_path(&app);
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
        app.on_select_render_video_rgb_mode(move |mode| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = match mode.as_str() {
                "straight" => "straight",
                _ => "premultiplied",
            };
            app.set_render_video_rgb_mode_text(mode.into());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_toggle_render_export_alpha_mask(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_export_alpha_mask(!app.get_render_export_alpha_mask());
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
            let suggested_output = current_export_output_path(&app).unwrap_or_else(|_| {
                let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
                let audio_format =
                    AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
                let video_container = video_output_container_from_text(
                    app.get_render_video_container_text().as_str(),
                );
                PathBuf::from(default_render_output_path(
                    app.get_selected_midi_name().as_str(),
                    mode,
                    audio_format,
                    video_container,
                ))
            });
            let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
            let audio_format =
                AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
            let video_container =
                video_output_container_from_text(app.get_render_video_container_text().as_str());
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new();
                if let Some(parent) = suggested_output.parent() {
                    dialog = dialog.set_directory(parent);
                }
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                dialog = match mode {
                    RenderExportMode::VideoAudio | RenderExportMode::VideoOnly => dialog
                        .add_filter(
                            match video_container {
                                meridian_core::protocol::VideoOutputContainer::Mp4 => "MP4",
                                meridian_core::protocol::VideoOutputContainer::Mkv => "MKV",
                            },
                            &[video_container.extension()],
                        ),
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
                    app.set_render_output_path_text(path.display().to_string().into());
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
            let draft = match RenderExportDraft::from_app(&app) {
                Ok(draft) => draft,
                Err(message) => {
                    set_export_status(&app, "Failed", message, 0.0);
                    app.window().request_redraw();
                    return;
                }
            };

            {
                let mut controller = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                if controller.is_active() {
                    return;
                }
                if controller.begin(draft.clone()).is_err() {
                    return;
                }
            }

            app.set_render_output_path_text(draft.final_output.display().to_string().into());

            set_export_status(
                &app,
                "Preparing export",
                draft.final_output.display().to_string(),
                0.0,
            );
            if let Err(message) =
                start_render_export_jobs(&app, &bridge, &shared_state, &export_state, &draft)
            {
                export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned")
                    .clear();
                set_export_status(&app, "Failed", message, 0.0);
                app.window().request_redraw();
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

            let active = {
                let controller = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                controller.is_active()
            };
            if !active {
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
                export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned")
                    .clear();
                set_export_status(&app, "Cancelled", "Render cancelled".to_string(), 0.0);
            }
            app.window().request_redraw();
        });
    }
}
