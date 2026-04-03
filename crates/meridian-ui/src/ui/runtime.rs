use std::{
    cell::RefCell,
    ffi::OsStr,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use meridian_core::{
    audio::{
        AudioConfig, ChannelCount, EnvelopeCurveType, Interpolator, MeridianSoundfont, ThreadCount,
        DEFAULT_SOUNDFONT,
    },
    display::MIN_VIEW_RANGE_SECONDS,
    midi::MidiProcessingConfig,
    protocol::{CoreEvent, ParsedMidiId, ProcessedMidiId},
    render::{
        DisplayTimeSpace, KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig,
        NoteProjectorConfig, PfaKeyboardProjectorConfig, PianoTrailClassicSceneConfig,
        ProjectorImageConfig, RendererKind, SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
        PFA_BLUE_TOP_BAR_COLOR, PFA_GREEN_TOP_BAR_COLOR, PFA_RED_TOP_BAR_COLOR,
    },
    spawn_core, CoreHandle, MeridianError,
};
use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use slint::ComponentHandle;

use super::{
    core_bridge::UiCoreBridge,
    state::{apply_events_to_app, UiOptions},
    view::{App, MidiLoadState},
    view_model::UiViewModel,
    viewport::ViewportRenderer,
};

static LAST_PALETTE_PNG: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static LAST_AURA_PNG: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub fn run_ui(options: UiOptions) -> Result<(), MeridianError> {
    let backend_selector = slint::BackendSelector::new();
    if options.disable_wgpu {
        backend_selector
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    } else {
        backend_selector
            .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    }

    let app = App::new().map_err(|e| MeridianError::Platform(e.to_string()))?;
    let bridge = UiCoreBridge::new(spawn_core());
    let shared_state = Arc::new(Mutex::new(UiViewModel::default()));
    let preview_load_generation = Arc::new(AtomicU64::new(0));
    let render_load_generation = Arc::new(AtomicU64::new(0));
    let audio_load_generation = Arc::new(AtomicU64::new(0));
    let analysis_load_generation = Arc::new(AtomicU64::new(0));
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&bridge, &options, &app, &shared_state)?;
    install_load_progress_listener(&app, bridge.core());
    wire_callbacks(
        &app,
        &bridge,
        &shared_state,
        &preview_load_generation,
        &render_load_generation,
        &audio_load_generation,
        &analysis_load_generation,
    );
    install_drag_drop(&app);
    install_viewport(
        &app,
        bridge.core(),
        &shared_state,
        &pending_viewport_image,
        &viewport_size,
        options.disable_wgpu,
    )?;
    let _animation_timer = install_timer(
        &app,
        &bridge,
        &shared_state,
        &pending_viewport_image,
        &viewport_size,
        options.disable_wgpu,
    );

    app.window().request_redraw();
    app.run()
        .map_err(|e| MeridianError::Platform(e.to_string()))
}

pub(crate) fn initialize_core(
    bridge: &UiCoreBridge,
    options: &UiOptions,
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
) -> Result<(), MeridianError> {
    if let Some(path) = &options.midi_path {
        let name: slint::SharedString = path.display().to_string().into();
        set_selected_midi(app, name);
    }
    bridge.initialize(options, shared_state)?;
    let initial = bridge.refresh_state(shared_state)?;
    apply_events_to_app(app, shared_state, &initial);
    Ok(())
}

fn wire_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    wire_transport_callbacks(app, bridge, shared_state);
    wire_video_callbacks(app, bridge, shared_state);
    wire_audio_config_callbacks(app, bridge, shared_state);
    wire_midi_callbacks(
        app,
        bridge,
        shared_state,
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
}

fn app_is_loading(app: &App) -> bool {
    app.get_render_load_state() == MidiLoadState::Loading
        || app.get_audio_load_state() == MidiLoadState::Loading
        || app.get_analysis_load_state() == MidiLoadState::Loading
}

fn cancel_pending_midi_loads(
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

fn reset_midi_ui_state(app: &App, state: MidiLoadState) {
    app.set_viewport_image(slint::Image::default());
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

fn unload_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, Default::default());
    reset_midi_ui_state(app, MidiLoadState::NoMidi);
    app.window().request_redraw();
}

fn replace_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
    selected_midi_name: slint::SharedString,
) {
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, selected_midi_name);
    reset_midi_ui_state(app, MidiLoadState::Selected);
    app.window().request_redraw();
}

fn update_video_scene(
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

fn update_video_key_range(
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

fn update_video_view_range(
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

fn update_audio_config(
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

fn primary_soundfont(config: &mut AudioConfig) -> &mut MeridianSoundfont {
    if config.soundfonts.is_empty() {
        config.soundfonts.push(MeridianSoundfont::default());
    }
    config
        .soundfonts
        .first_mut()
        .expect("audio config must always have a primary soundfont")
}

fn set_selected_midi(app: &App, selected_midi_name: slint::SharedString) {
    app.set_selected_midi_name(selected_midi_name.clone());
    app.set_window_title(window_title_for_selected(selected_midi_name.as_str()));
}

fn window_title_for_selected(selected_midi_name: &str) -> slint::SharedString {
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

fn wire_transport_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_step_time(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.step_time(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.zoom(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.toggle_play(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_seek_time(move |time| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.seek_time(time as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_renderer(move |renderer| {
            if let Some(app) = app_weak.upgrade() {
                let renderer = match renderer.as_str() {
                    "flat" => RendererKind::Flat,
                    "piano_trail_classic" | "3d" => RendererKind::PianoTrailClassic,
                    _ => RendererKind::Pfa,
                };
                if let Ok(events) = bridge.set_renderer(renderer, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_time_space(move |time_space| {
            if let Some(app) = app_weak.upgrade() {
                let time_space = match time_space.as_str() {
                    "tick" => DisplayTimeSpace::Tick,
                    _ => DisplayTimeSpace::Time,
                };
                if let Ok(events) = bridge.set_time_space(time_space, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
}

fn wire_video_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_control(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match key.as_str() {
                "view_range" => {
                    let Ok(seconds) = value.parse::<f64>() else {
                        return;
                    };
                    update_video_view_range(&app, &bridge, &shared_state, seconds);
                }
                "first_key" => {
                    let Ok(first_key) = value.parse::<u8>() else {
                        return;
                    };
                    let last_key = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.last_key)
                        .unwrap_or(127);
                    update_video_key_range(&app, &bridge, &shared_state, first_key, last_key);
                }
                "last_key" => {
                    let Ok(last_key) = value.parse::<u8>() else {
                        return;
                    };
                    let first_key = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.first_key)
                        .unwrap_or(0);
                    update_video_key_range(&app, &bridge, &shared_state, first_key, last_key);
                }
                "keyboard_height_mode" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let SceneConfig::TwoD(config) = scene else {
                            return;
                        };
                        let current = match config.keyboard_height {
                            KeyboardHeightSpec::ScreenPercent { height } => height,
                            KeyboardHeightSpec::AspectRatio { ratio } => ratio,
                        };
                        config.keyboard_height = if value.as_str() == "screen_percent" {
                            KeyboardHeightSpec::ScreenPercent { height: current }
                        } else {
                            KeyboardHeightSpec::AspectRatio { ratio: current }
                        };
                    });
                }
                "keyboard_height_value" => {
                    let Ok(parsed) = value.parse::<f32>() else {
                        return;
                    };
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let SceneConfig::TwoD(config) = scene else {
                            return;
                        };
                        config.keyboard_height = match config.keyboard_height {
                            KeyboardHeightSpec::ScreenPercent { .. } => {
                                KeyboardHeightSpec::ScreenPercent { height: parsed }
                            }
                            KeyboardHeightSpec::AspectRatio { .. } => {
                                KeyboardHeightSpec::AspectRatio { ratio: parsed }
                            }
                        };
                    });
                }
                "pfa_note_same_width" => update_pfa_note_bool(
                    &app,
                    &bridge,
                    &shared_state,
                    value.as_str(),
                    |config, enabled| config.same_width_notes = enabled,
                ),
                "pfa_border_width" => {
                    let Ok(parsed) = value.parse::<f32>() else {
                        return;
                    };
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let SceneConfig::TwoD(config) = scene {
                            if let NoteProjectorConfig::Pfa(notes) = &mut config.notes {
                                notes.border_width = parsed;
                            }
                        }
                    });
                }
                "pfa_keyboard_same_width" => update_pfa_keyboard_bool(
                    &app,
                    &bridge,
                    &shared_state,
                    value.as_str(),
                    |config, enabled| config.same_width_notes = enabled,
                ),
                "pfa_middle_c" => update_pfa_keyboard_bool(
                    &app,
                    &bridge,
                    &shared_state,
                    value.as_str(),
                    |config, enabled| config.middle_c = enabled,
                ),
                "pfa_top_color" => {
                    let color = match value.as_str() {
                        "blue" => PFA_BLUE_TOP_BAR_COLOR,
                        "green" => PFA_GREEN_TOP_BAR_COLOR,
                        _ => PFA_RED_TOP_BAR_COLOR,
                    };
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let SceneConfig::TwoD(config) = scene {
                            if let KeyboardProjectorConfig::Pfa(keyboard) = &mut config.keyboard {
                                keyboard.top_bar_color = color.to_string();
                            }
                        }
                    });
                }
                "pfa_top_bar_color" => {
                    let Some(color) = normalize_top_bar_color(value.as_str()) else {
                        return;
                    };
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let SceneConfig::TwoD(config) = scene {
                            if let KeyboardProjectorConfig::Pfa(keyboard) = &mut config.keyboard {
                                keyboard.top_bar_color = color.clone();
                            }
                        }
                    });
                }
                "palette_source" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(palette) = palette_mut(scene) else {
                            return;
                        };
                        *palette = if value.as_str() == "zenith_palette" {
                            NotePaletteConfig::ZenithPalette {
                                palette: ZenithPaletteSpec::Random,
                                randomize: true,
                            }
                        } else {
                            NotePaletteConfig::DefaultTrackColors
                        };
                    });
                }
                "palette_kind" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                            palette_mut(scene)
                        else {
                            return;
                        };
                        *palette = match value.as_str() {
                            "random_gradients" => ZenithPaletteSpec::RandomGradients,
                            "png_file" => match LAST_PALETTE_PNG
                                .lock()
                                .expect("palette png mutex poisoned")
                                .clone()
                            {
                                Some(path) => ZenithPaletteSpec::PngFile { path },
                                None => return,
                            },
                            _ => ZenithPaletteSpec::Random,
                        };
                    });
                }
                "palette_randomize" => {
                    let enabled = bool_from_label(value.as_str());
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(NotePaletteConfig::ZenithPalette { randomize, .. }) =
                            palette_mut(scene)
                        {
                            *randomize = enabled;
                        }
                    });
                }
                "ptc_same_width_notes" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.same_width_notes = v
                    })
                }
                "ptc_vertical_notes" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.vertical_notes = v
                    })
                }
                "ptc_box_notes" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.box_notes = v
                    })
                }
                "ptc_light_shade" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.light_shade = v
                    })
                }
                "ptc_show_keyboard" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.show_keyboard = v
                    })
                }
                "ptc_tilt_keys" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.tilt_keys = v
                    })
                }
                "ptc_eat_notes" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.eat_notes = v
                    })
                }
                "ptc_aura_enabled" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.aura_enabled = v
                    })
                }
                "ptc_notes_change_size" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.notes_change_size = v
                    })
                }
                "ptc_notes_change_tint" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.notes_change_tint = v
                    })
                }
                "ptc_use_vel" => {
                    update_ptc_bool(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.use_vel = v
                    })
                }
                "ptc_fov" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.fov = v.to_radians()
                    })
                }
                "ptc_view_height" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.view_height = v
                    })
                }
                "ptc_view_offset" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.view_offset = v
                    })
                }
                "ptc_view_pan" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.view_pan = v
                    })
                }
                "ptc_cam_ang" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.cam_ang = v.to_radians()
                    })
                }
                "ptc_cam_rot" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.cam_rot = v.to_radians()
                    })
                }
                "ptc_cam_spin" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.cam_spin = v.to_radians()
                    })
                }
                "ptc_viewdist" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.viewdist = v
                    })
                }
                "ptc_viewback" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.viewback = v
                    })
                }
                "ptc_note_down_speed" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.note_down_speed = v
                    })
                }
                "ptc_note_up_speed" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.note_up_speed = v
                    })
                }
                "ptc_aura_strength" => {
                    update_ptc_f32(&app, &bridge, &shared_state, value.as_str(), |c, v| {
                        c.aura_strength = v
                    })
                }
                "ptc_aura_image_source" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(config) = ptc_mut(scene) else {
                            return;
                        };
                        config.aura_image = if value.as_str() == "builtin" {
                            ProjectorImageConfig::Builtin {
                                name: "ring".into(),
                            }
                        } else {
                            match LAST_AURA_PNG
                                .lock()
                                .expect("aura png mutex poisoned")
                                .clone()
                            {
                                Some(path) => ProjectorImageConfig::PngFile { path },
                                None => return,
                            }
                        };
                    });
                }
                "ptc_aura_image_builtin" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(config) = ptc_mut(scene) else {
                            return;
                        };
                        config.aura_image = ProjectorImageConfig::Builtin {
                            name: value.to_string(),
                        };
                    });
                }
                _ => {}
            }
        });
    }
    // ── Float-valued slider callback ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_update_video_control_float(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match key.as_str() {
                "view_range" => {
                    update_video_view_range(&app, &bridge, &shared_state, value as f64);
                }
                "keyboard_height_value" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let SceneConfig::TwoD(config) = scene else {
                            return;
                        };
                        config.keyboard_height = match config.keyboard_height {
                            KeyboardHeightSpec::ScreenPercent { .. } => {
                                KeyboardHeightSpec::ScreenPercent { height: parsed }
                            }
                            KeyboardHeightSpec::AspectRatio { .. } => {
                                KeyboardHeightSpec::AspectRatio { ratio: parsed }
                            }
                        };
                    });
                }
                "pfa_border_width" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let SceneConfig::TwoD(config) = scene {
                            if let NoteProjectorConfig::Pfa(notes) = &mut config.notes {
                                notes.border_width = parsed;
                            }
                        }
                    });
                }
                "ptc_fov" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(config) = ptc_mut(scene) {
                            config.fov = parsed.to_radians();
                        }
                    });
                }
                "ptc_view_height" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.view_height = parsed;
                        }
                    });
                }
                "ptc_view_offset" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.view_offset = parsed;
                        }
                    });
                }
                "ptc_view_pan" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.view_pan = parsed;
                        }
                    });
                }
                "ptc_cam_ang" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.cam_ang = parsed.to_radians();
                        }
                    });
                }
                "ptc_cam_rot" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.cam_rot = parsed.to_radians();
                        }
                    });
                }
                "ptc_cam_spin" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.cam_spin = parsed.to_radians();
                        }
                    });
                }
                "ptc_viewdist" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.viewdist = parsed;
                        }
                    });
                }
                "ptc_viewback" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.viewback = parsed;
                        }
                    });
                }
                "ptc_note_down_speed" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.note_down_speed = parsed;
                        }
                    });
                }
                "ptc_note_up_speed" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.note_up_speed = parsed;
                        }
                    });
                }
                "ptc_aura_strength" => {
                    let parsed = value;
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(c) = ptc_mut(scene) {
                            c.aura_strength = parsed;
                        }
                    });
                }
                _ => {}
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_browse_video_asset(move |target| {
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("PNG", &["png", "PNG"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let target = target.to_string();
                    let _ = app_weak.upgrade_in_event_loop(move |app| match target.as_str() {
                        "palette_png" => {
                            *LAST_PALETTE_PNG.lock().expect("palette png mutex poisoned") =
                                Some(path.clone());
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                if let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                                    palette_mut(scene)
                                {
                                    *palette = ZenithPaletteSpec::PngFile { path };
                                }
                            });
                        }
                        "aura_png" => {
                            *LAST_AURA_PNG.lock().expect("aura png mutex poisoned") =
                                Some(path.display().to_string());
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                if let Some(config) = ptc_mut(scene) {
                                    config.aura_image = ProjectorImageConfig::PngFile {
                                        path: path.display().to_string(),
                                    };
                                }
                            });
                        }
                        _ => {}
                    });
                }
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_reset_video_asset(move |target| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match target.as_str() {
                "palette_png" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                            palette_mut(scene)
                        {
                            *palette = ZenithPaletteSpec::Random;
                        }
                    });
                }
                "aura_png" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let Some(config) = ptc_mut(scene) {
                            config.aura_image = ProjectorImageConfig::Builtin {
                                name: "ring".into(),
                            };
                        }
                    });
                }
                _ => {}
            }
        });
    }
}

fn wire_audio_config_callbacks(
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
                    config.xsynth.config.ignore_range = match limit.as_str() {
                        "8" => 1..=8,
                        "16" => 1..=16,
                        "24" => 1..=24,
                        _ => 0..=0,
                    };
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

fn bool_from_label(label: &str) -> bool {
    matches!(label, "on" | "yes" | "true" | "1")
}

fn normalize_top_bar_color(value: &str) -> Option<String> {
    PfaKeyboardProjectorConfig::normalize_top_bar_color(value)
}

fn palette_mut(scene: &mut SceneConfig) -> Option<&mut NotePaletteConfig> {
    match scene {
        SceneConfig::TwoD(config) => match &mut config.notes {
            NoteProjectorConfig::Flat(notes) => Some(&mut notes.palette),
            NoteProjectorConfig::Pfa(notes) => Some(&mut notes.palette),
        },
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            Some(&mut config.palette)
        }
    }
}

fn ptc_mut(scene: &mut SceneConfig) -> Option<&mut PianoTrailClassicSceneConfig> {
    match scene {
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => Some(config),
        _ => None,
    }
}

fn update_pfa_note_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut meridian_core::render::PfaNoteProjectorConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let SceneConfig::TwoD(config) = scene {
            if let NoteProjectorConfig::Pfa(notes) = &mut config.notes {
                mutate(notes, enabled);
            }
        }
    });
}

fn update_pfa_keyboard_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut meridian_core::render::PfaKeyboardProjectorConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let SceneConfig::TwoD(config) = scene {
            if let KeyboardProjectorConfig::Pfa(keyboard) = &mut config.keyboard {
                mutate(keyboard, enabled);
            }
        }
    });
}

fn update_ptc_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut PianoTrailClassicSceneConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let Some(config) = ptc_mut(scene) {
            mutate(config, enabled);
        }
    });
}

fn update_ptc_f32(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut PianoTrailClassicSceneConfig, f32) + 'static,
) {
    let Ok(parsed) = value.parse::<f32>() else {
        return;
    };
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let Some(config) = ptc_mut(scene) {
            mutate(config, parsed);
        }
    });
}

/// Wire MIDI file selection, loading, unloading, and drag-drop callbacks.
fn wire_midi_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    // ── Browse button (open native file dialog) ──
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
            // rfd's async dialog won't block the event loop on supported platforms
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

    // ── File dropped from OS ──
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
                // Auto-load for the current profile context
                match app.get_active_profile() {
                    0 => load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &path,
                    ),
                    1 => load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &path,
                    ),
                    2 => load_audio_midi_async(
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

    // ── Load for preview ──
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

    // ── Load for render ──
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

    // ── Load for audio ──
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

    // ── Load for analysis / modify ──
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

    // ── Cancel preview load ──
    {
        let app_weak = app.as_weak();
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_cancel_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
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

    // ── Cancel render load ──
    {
        let app_weak = app.as_weak();
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_cancel_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
            }
        });
    }

    // ── Cancel analysis load ──
    {
        let app_weak = app.as_weak();
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_cancel_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_analysis_load_state() == MidiLoadState::Loading {
                    app.set_analysis_load_state(MidiLoadState::Selected);
                }
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
            }
        });
    }

    // ── Cancel audio load ──
    {
        let app_weak = app.as_weak();
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_cancel_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    // ── Unload preview ──
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

    // ── Unload render ──
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

    // ── Unload audio ──
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

    // ── Unload analysis ──
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

    // ── Retry load preview ──
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

    // ── Retry load render ──
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

    // ── Retry load audio ──
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

    // ── Retry load analysis ──
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

/// Start a preview MIDI load without blocking the Slint event loop.
fn load_preview_midi_async(
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
        let result = (|| -> Result<Vec<CoreEvent>, MeridianError> {
            let mut events = bridge.load_midi(path, &shared_state)?;
            events.extend(bridge.set_playing(true, &shared_state)?);
            Ok(events)
        })();
        if preview_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if preview_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
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
            }
            app.window().request_redraw();
        });
    });
}

/// Start a render-context MIDI load without blocking the Slint event loop.
fn load_render_midi_async(
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
        let result = bridge.load_display_midi(path, &shared_state);
        if render_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if render_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
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
            }
            app.window().request_redraw();
        });
    });
}

fn load_audio_midi_async(
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
        let result = bridge.load_audio_midi(path, &shared_state);
        if audio_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if audio_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
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
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_midi_async(
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
        let result = load_analysis_resource_set(&bridge, &shared_state, &path, progress);
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
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
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_resource_set(
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    path: &std::path::Path,
    mut progress: impl FnMut(f32, &'static str),
) -> Result<Vec<CoreEvent>, MeridianError> {
    let mut all_events = Vec::new();

    let parsed_events = bridge.load_parsed_midi(path.to_path_buf(), shared_state)?;
    let parsed_midi_id = parsed_midi_id_from_events(&parsed_events)?;
    all_events.extend(parsed_events);

    progress(0.55, "Building analysis model…");
    let processed_events = bridge.build_processed_midi(
        parsed_midi_id,
        MidiProcessingConfig::default(),
        shared_state,
    )?;
    let (processed_midi_id, midi_length) = processed_result_from_events(&processed_events)?;
    all_events.extend(processed_events);

    let bucket_count = ((midi_length / 0.5).ceil() as usize).clamp(1, 8192);
    progress(0.82, "Computing bucketed note statistics…");
    let analysis_events =
        bridge.analyze_processed_midi(processed_midi_id, Some(bucket_count), shared_state)?;
    all_events.extend(analysis_events);

    Ok(all_events)
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

fn processed_result_from_events(
    events: &[CoreEvent],
) -> Result<(ProcessedMidiId, f64), MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ProcessedMidiBuilt {
                processed_midi_id,
                midi_length,
                ..
            } => Some((*processed_midi_id, *midi_length)),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing processed MIDI id".into()))
}

fn reset_analysis_outputs(app: &App) {
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

/// Install the winit window event handler for OS file drag-and-drop.
fn install_drag_drop(app: &App) {
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

fn install_load_progress_listener(app: &App, core: &CoreHandle) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    std::thread::spawn(move || {
        for event in receiver {
            let CoreEvent::MidiLoadProgress {
                path,
                progress,
                status,
            } = event
            else {
                continue;
            };

            let path_text: slint::SharedString = path.display().to_string().into();
            let status_text: slint::SharedString = status.into();
            let _ = app_weak.upgrade_in_event_loop(move |app| {
                if app.get_selected_midi_name() != path_text {
                    return;
                }
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_loading_progress(progress);
                    app.set_render_loading_status(status_text.clone());
                }
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_loading_progress(progress);
                    app.set_audio_loading_status(status_text);
                }
                app.window().request_redraw();
            });
        }
    });
}

fn install_viewport(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> Result<(), MeridianError> {
    if disable_wgpu {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
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

fn install_timer(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> slint::Timer {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let bridge_for_timer = bridge.clone();
    let shared_state_for_timer = Arc::clone(shared_state);
    let pending_viewport_image_for_timer = Rc::clone(pending_viewport_image);
    let viewport_size_for_timer = Rc::clone(viewport_size);

    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(app) = app_for_timer.upgrade() {
                *viewport_size_for_timer.borrow_mut() = (
                    app.get_viewport_px_width().max(1.0) as u32,
                    app.get_viewport_px_height().max(1.0) as u32,
                );
                if let Some(image) = pending_viewport_image_for_timer.borrow_mut().take() {
                    app.set_viewport_image(image);
                }
                if !app_is_loading(&app) {
                    if let Ok(events) = bridge_for_timer.refresh_state(&shared_state_for_timer) {
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
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );
    animation_timer
}
