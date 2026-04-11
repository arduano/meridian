use slint::ComponentHandle;

use meridian_core::{display::MIN_VIEW_RANGE_SECONDS, render::SceneLayout};

use super::{schema::UiConfigFile, App, UiOptions, UiStartupOptions};

pub(in super::super) fn build_startup_options(
    launch: &UiOptions,
    persisted: Option<&UiConfigFile>,
) -> UiStartupOptions {
    let mut startup = UiStartupOptions {
        disable_wgpu: launch.disable_wgpu,
        ..UiStartupOptions::default()
    };

    if let Some(config) = persisted {
        startup.audio = config.preferences.audio.clone();
        startup.scene = config.preferences.scene.clone();
        startup.view_range = config.preferences.view_range;
        startup.time_space = config.preferences.time_space;
        startup.first_key = config.preferences.first_key;
        startup.last_key = config.preferences.last_key;

        if config.preferences.reopen_last_session {
            startup.midi_path = config
                .session
                .midi_path
                .as_ref()
                .filter(|path| path.exists())
                .cloned();
            startup.start_time = config.session.current_time.max(0.0);
        }
    }

    if let Some(renderer) = launch.renderer {
        let mut layout = SceneLayout {
            scene: startup.scene.clone(),
            ..SceneLayout::default()
        };
        layout.set_renderer_kind(renderer);
        startup.scene = layout.scene;
    }
    if let Some(view_range) = launch.view_range {
        startup.view_range = view_range;
    }
    if let Some(first_key) = launch.first_key {
        startup.first_key = first_key;
    }
    if let Some(last_key) = launch.last_key {
        startup.last_key = last_key;
    }
    if let Some(midi_path) = &launch.midi_path {
        startup.midi_path = Some(midi_path.clone());
        startup.start_time = launch.start_time.unwrap_or(0.0).max(0.0);
    } else if let Some(start_time) = launch.start_time {
        startup.start_time = start_time.max(0.0);
    }

    startup.first_key = startup.first_key.min(startup.last_key);
    startup.last_key = startup.last_key.max(startup.first_key);
    startup.view_range = startup.view_range.max(MIN_VIEW_RANGE_SECONDS);
    startup
}

pub(in super::super) fn apply_persisted_window_preferences(
    app: &App,
    config: Option<&UiConfigFile>,
) {
    let Some(window) = config.map(|config| &config.preferences.window) else {
        return;
    };
    if let Some(size) = window.size {
        app.window().set_size(slint::PhysicalSize::new(
            size.width.max(1),
            size.height.max(1),
        ));
    }
    if let Some(position) = window.position {
        app.window()
            .set_position(slint::PhysicalPosition::new(position.x, position.y));
    }
    if window.maximized {
        app.window().set_maximized(true);
    }
    if window.fullscreen {
        app.window().set_fullscreen(true);
    }
}
