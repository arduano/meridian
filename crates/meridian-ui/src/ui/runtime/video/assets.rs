use super::*;

pub(super) fn wire_video_asset_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
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
                        "background_png" => {
                            *LAST_BACKGROUND_PNG
                                .lock()
                                .expect("background png mutex poisoned") =
                                Some(path.display().to_string());
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                let scaling = match background_mut(scene) {
                                    ProjectorBackgroundConfig::PngFile { scaling, .. } => *scaling,
                                    ProjectorBackgroundConfig::None => {
                                        ProjectorBackgroundScalingMode::Stretch
                                    }
                                };
                                *background_mut(scene) = ProjectorBackgroundConfig::PngFile {
                                    path: path.display().to_string(),
                                    scaling,
                                };
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
                "background_png" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        *background_mut(scene) = ProjectorBackgroundConfig::None;
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
