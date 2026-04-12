use super::*;

pub(super) fn wire_video_control_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
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
                                randomize: false,
                            }
                        } else {
                            NotePaletteConfig::DefaultTrackColors
                        };
                    });
                }
                "palette_kind" => {
                    let palette_png = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .remembered_assets
                        .palette_png
                        .clone();
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                            palette_mut(scene)
                        else {
                            return;
                        };
                        *palette = match value.as_str() {
                            "random_gradients" => ZenithPaletteSpec::RandomGradients,
                            "png_file" => match palette_png {
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
                "background_source" => {
                    let background_png = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .remembered_assets
                        .background_png
                        .clone();
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        *background_mut(scene) = if value.as_str() == "png_file" {
                            match background_png {
                                Some(path) => ProjectorBackgroundConfig::PngFile {
                                    path,
                                    scaling: ProjectorBackgroundScalingMode::Stretch,
                                },
                                None => return,
                            }
                        } else {
                            ProjectorBackgroundConfig::None
                        };
                    });
                }
                "background_scaling" => {
                    let scaling = if value.as_str() == "cover" {
                        ProjectorBackgroundScalingMode::Cover
                    } else {
                        ProjectorBackgroundScalingMode::Stretch
                    };
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        if let ProjectorBackgroundConfig::PngFile {
                            scaling: current, ..
                        } = background_mut(scene)
                        {
                            *current = scaling;
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
                    let aura_png = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .remembered_assets
                        .aura_png
                        .clone();
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(config) = ptc_mut(scene) else {
                            return;
                        };
                        config.aura_image = if value.as_str() == "builtin" {
                            ProjectorImageConfig::Builtin {
                                name: "ring".into(),
                            }
                        } else {
                            match aura_png {
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
}

pub(super) fn bool_from_label(label: &str) -> bool {
    matches!(label, "on" | "yes" | "true" | "1")
}

pub(super) fn normalize_top_bar_color(value: &str) -> Option<String> {
    PfaKeyboardProjectorConfig::normalize_top_bar_color(value)
}

pub(super) fn palette_mut(scene: &mut SceneConfig) -> Option<&mut NotePaletteConfig> {
    match scene {
        SceneConfig::TwoD(config) => match &mut config.notes {
            NoteProjectorConfig::Flat(notes) => Some(&mut notes.palette),
            NoteProjectorConfig::Pfa(notes) => Some(&mut notes.palette),
        },
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            Some(&mut config.palette)
        }
        SceneConfig::Text(_) => None,
    }
}

pub(super) fn background_mut(scene: &mut SceneConfig) -> &mut ProjectorBackgroundConfig {
    scene.background_mut()
}

pub(super) fn ptc_mut(scene: &mut SceneConfig) -> Option<&mut PianoTrailClassicSceneConfig> {
    match scene {
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => Some(config),
        _ => None,
    }
}

pub(super) fn update_pfa_note_bool(
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

pub(super) fn update_pfa_keyboard_bool(
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

pub(super) fn update_ptc_bool(
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

pub(super) fn update_ptc_f32(
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
