use super::*;

pub(super) fn wire_audio_config_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_sample_rate(move |rate| {
            let Ok(sample_rate) = rate.parse::<u32>() else {
                return;
            };
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.audio_params.sample_rate = sample_rate;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_channel_count(move |channels| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.audio_params.channels = if channels.as_str() == "mono" {
                        ChannelCount::Mono
                    } else {
                        ChannelCount::Stereo
                    };
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_render_window(move |window_ms| {
            let Ok(render_window_ms) = window_ms.parse::<f64>() else {
                return;
            };
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.config.render_window_ms = render_window_ms;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_soundfont(move || {
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("Soundfonts", &["sf2", "sfz", "SF2", "SFZ"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        update_audio_config(&app, &bridge, &shared_state, move |config| {
                            let soundfont = primary_soundfont(config);
                            soundfont.path = path;
                            soundfont.enabled = true;
                        });
                    });
                }
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_reset_audio_soundfont(move || {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let soundfont = primary_soundfont(config);
                    soundfont.path = PathBuf::from(DEFAULT_SOUNDFONT);
                    soundfont.enabled = true;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_audio_soundfont_enabled(move || {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let soundfont = primary_soundfont(config);
                    soundfont.enabled = !soundfont.enabled;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_interpolation(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.interpolator = if mode.as_str() == "linear" {
                            Interpolator::Linear
                        } else {
                            Interpolator::Nearest
                        };
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_effects(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.use_effects = mode.as_str() == "on";
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_layer_limit(move |limit| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    if limit.as_str() == "off" {
                        config.xsynth.limit_layers = false;
                    } else if let Ok(layers) = limit.parse::<usize>() {
                        config.xsynth.limit_layers = true;
                        config.xsynth.layers = layers;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_threading(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.config.multithreading = match mode.as_str() {
                        "auto" => ThreadCount::Auto,
                        "4" => ThreadCount::Manual(4),
                        _ => ThreadCount::None,
                    };
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_ignore_range(move |limit| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    if let Some(ignore_range) = ignore_range_from_keep_velocity(limit.as_str()) {
                        config.xsynth.config.ignore_range = ignore_range;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_attack_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.attack_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_decay_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.decay_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_release_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.release_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_limiter(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.use_limiter = mode.as_str() == "on";
                });
            }
        });
    }
}

fn envelope_curve_from_label(label: &str) -> EnvelopeCurveType {
    match label {
        "linear" => EnvelopeCurveType::Linear,
        _ => EnvelopeCurveType::Exponential,
    }
}

fn ignore_range_from_keep_velocity(label: &str) -> Option<std::ops::RangeInclusive<u8>> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("off") {
        return Some(0..=0);
    }

    let keep_from = trimmed.parse::<u8>().ok()?.clamp(1, 127);
    if keep_from <= 1 {
        Some(0..=0)
    } else {
        Some(1..=keep_from - 1)
    }
}
