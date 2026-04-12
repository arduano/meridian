use super::*;
use super::super::persistence::assets::{
    remember_aura_png, remember_background_png, remember_palette_png,
};

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
                            {
                                let mut state = shared_state
                                    .lock()
                                    .expect("shared UI state mutex poisoned");
                                remember_palette_png(&mut state, path.clone());
                            }
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                if let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                                    palette_mut(scene)
                                {
                                    *palette = ZenithPaletteSpec::PngFile { path };
                                }
                            });
                        }
                        "background_png" => {
                            let path = path.display().to_string();
                            {
                                let mut state = shared_state
                                    .lock()
                                    .expect("shared UI state mutex poisoned");
                                remember_background_png(&mut state, path.clone());
                            }
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                let scaling = match background_mut(scene) {
                                    ProjectorBackgroundConfig::PngFile { scaling, .. } => *scaling,
                                    ProjectorBackgroundConfig::None => {
                                        ProjectorBackgroundScalingMode::Stretch
                                    }
                                };
                                *background_mut(scene) = ProjectorBackgroundConfig::PngFile {
                                    path,
                                    scaling,
                                };
                            });
                        }
                        "aura_png" => {
                            let path = path.display().to_string();
                            {
                                let mut state = shared_state
                                    .lock()
                                    .expect("shared UI state mutex poisoned");
                                remember_aura_png(&mut state, path.clone());
                            }
                            update_video_scene(&app, &bridge, &shared_state, move |scene| {
                                if let Some(config) = ptc_mut(scene) {
                                    config.aura_image = ProjectorImageConfig::PngFile { path };
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
