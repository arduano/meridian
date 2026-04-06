use super::*;

pub(super) fn wire_video_float_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
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
}
