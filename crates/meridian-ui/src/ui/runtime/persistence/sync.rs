use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::ComponentHandle;

use super::{
    append_merge_source_paths, apply_events_to_app, apply_merge_sources_to_app,
    load_modify_pass_into_app, validate_modify_config, App, Arc, Mutex, UiCoreBridge, UiViewModel,
};

use super::{
    assets::{
        active_aura_path_from_scene, active_background_path_from_scene,
        active_palette_path_from_scene, file_name_or_path, last_aura_png, last_background_png,
        last_palette_png, restore_last_asset_paths,
    },
    schema::{
        ExportPreferences, MergePreferences, ModifyPreferences, UiConfigFile, WindowPosition,
        WindowPreferences, WindowSize,
    },
    store::save_ui_config_to_dir,
};

const AUTOSAVE_POLL_INTERVAL: Duration = Duration::from_millis(750);
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(1250);

macro_rules! capture_text_fields {
    ($app:expr, $prefs:expr, $( $field:ident => ($getter:ident, $setter:ident) ),+ $(,)?) => {
        $(
            $prefs.$field = $app.$getter().to_string();
        )+
    };
}

macro_rules! restore_text_fields {
    ($app:expr, $prefs:expr, $( $field:ident => ($getter:ident, $setter:ident) ),+ $(,)?) => {
        $(
            $app.$setter($prefs.$field.clone().into());
        )+
    };
}

macro_rules! capture_bool_fields {
    ($app:expr, $prefs:expr, $( $field:ident => ($getter:ident, $setter:ident) ),+ $(,)?) => {
        $(
            $prefs.$field = $app.$getter();
        )+
    };
}

macro_rules! restore_bool_fields {
    ($app:expr, $prefs:expr, $( $field:ident => ($getter:ident, $setter:ident) ),+ $(,)?) => {
        $(
            $app.$setter($prefs.$field);
        )+
    };
}

macro_rules! export_text_bindings {
    ($macro:ident, $app:expr, $prefs:expr) => {
        $macro!(
            $app,
            $prefs,
            mode_text => (get_render_mode_text, set_render_mode_text),
            video_resolution_text => (
                get_render_video_resolution_text,
                set_render_video_resolution_text
            ),
            video_fps_text => (get_render_video_fps_text, set_render_video_fps_text),
            audio_format_text => (get_render_audio_format_text, set_render_audio_format_text),
            audio_sample_rate_text => (
                get_render_audio_sample_rate_text,
                set_render_audio_sample_rate_text
            ),
            audio_channel_count_text => (
                get_render_audio_channel_count_text,
                set_render_audio_channel_count_text
            ),
            video_ffmpeg_args_text => (
                get_render_video_ffmpeg_args_text,
                set_render_video_ffmpeg_args_text
            ),
            audio_ffmpeg_args_text => (
                get_render_audio_ffmpeg_args_text,
                set_render_audio_ffmpeg_args_text
            ),
            video_codec_text => (get_render_video_codec_text, set_render_video_codec_text),
            video_crf_text => (get_render_video_crf_text, set_render_video_crf_text),
            video_preset_text => (get_render_video_preset_text, set_render_video_preset_text),
            video_pix_fmt_text => (get_render_video_pix_fmt_text, set_render_video_pix_fmt_text),
            video_rgb_mode_text => (
                get_render_video_rgb_mode_text,
                set_render_video_rgb_mode_text
            ),
            audio_bitrate_text => (
                get_render_audio_bitrate_text,
                set_render_audio_bitrate_text
            )
        );
    };
}

macro_rules! export_bool_bindings {
    ($macro:ident, $app:expr, $prefs:expr) => {
        $macro!(
            $app,
            $prefs,
            export_alpha_mask => (
                get_render_export_alpha_mask,
                set_render_export_alpha_mask
            ),
            use_limiter => (get_render_use_limiter, set_render_use_limiter),
            open_after_export => (
                get_render_open_after_export,
                set_render_open_after_export
            )
        );
    };
}

macro_rules! merge_text_bindings {
    ($macro:ident, $app:expr, $prefs:expr) => {
        $macro!(
            $app,
            $prefs,
            layout_text => (get_merge_layout_text, set_merge_layout_text),
            metadata_mode_text => (get_merge_metadata_mode_text, set_merge_metadata_mode_text),
            ppq_override_text => (get_merge_ppq_override_text, set_merge_ppq_override_text)
        );
    };
}

#[derive(Debug, Clone)]
struct AutosaveState {
    seed_config: Option<UiConfigFile>,
    last_saved: Option<UiConfigFile>,
    last_observed: Option<UiConfigFile>,
    last_change_at: Option<Instant>,
}

impl AutosaveState {
    fn new(seed_config: Option<UiConfigFile>) -> Self {
        Self {
            seed_config: seed_config.clone(),
            last_saved: seed_config.clone(),
            last_observed: seed_config,
            last_change_at: None,
        }
    }
}

pub(in super::super) fn install_config_persistence_timer(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    initial_config: Option<UiConfigFile>,
) -> slint::Timer {
    let timer = slint::Timer::default();
    let autosave_state = Rc::new(RefCell::new(AutosaveState::new(initial_config)));
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);

    timer.start(
        slint::TimerMode::Repeated,
        AUTOSAVE_POLL_INTERVAL,
        move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let (seed_config, last_saved, last_observed, last_change_at) = {
                let autosave = autosave_state.borrow();
                (
                    autosave.seed_config.clone(),
                    autosave.last_saved.clone(),
                    autosave.last_observed.clone(),
                    autosave.last_change_at,
                )
            };

            let captured = capture_ui_config(&app, &shared_state, seed_config.as_ref());
            let now = Instant::now();
            let mut should_save = false;

            {
                let mut autosave = autosave_state.borrow_mut();
                if last_observed.as_ref() != Some(&captured) {
                    autosave.last_observed = Some(captured.clone());
                    autosave.last_change_at = Some(now);
                }

                if last_saved.as_ref() != Some(&captured)
                    && autosave.last_change_at.is_some_and(|changed_at| {
                        now.duration_since(changed_at) >= AUTOSAVE_DEBOUNCE
                    })
                {
                    should_save = true;
                }

                if last_change_at.is_none() && last_saved.is_none() {
                    should_save = true;
                }
            }

            if !should_save {
                return;
            }

            if save_ui_config_to_dir(None, &captured).is_ok() {
                let mut autosave = autosave_state.borrow_mut();
                autosave.last_saved = Some(captured);
                autosave.last_change_at = None;
            }
        },
    );

    timer
}

pub(in super::super) fn save_ui_config_now(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    seed_config: Option<&UiConfigFile>,
) -> Result<(), String> {
    let config = capture_ui_config(app, shared_state, seed_config);
    save_ui_config_to_dir(None, &config)
}

pub(in super::super) fn restore_persisted_ui_state(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    config: Option<&UiConfigFile>,
) {
    let Some(config) = config else {
        return;
    };

    app.set_active_profile(config.preferences.active_profile);
    restore_last_asset_paths(&config.preferences);
    restore_export_preferences(app, &config.preferences.export);
    restore_modify_preferences(app, &config.preferences.modify);
    restore_merge_preferences(app, &config.preferences.merge);

    if config.preferences.reopen_last_session {
        restore_merge_session(app, bridge, shared_state, &config.session);
    }

    if config.preferences.reopen_last_session
        && session_matches_current_midi(shared_state, config.session.midi_path.as_ref())
    {
        restore_selected_midi_session(app, bridge, shared_state, config);
    }
}

fn restore_export_preferences(app: &App, preferences: &ExportPreferences) {
    export_text_bindings!(restore_text_fields, app, preferences);
    export_bool_bindings!(restore_bool_fields, app, preferences);
}

fn restore_modify_preferences(app: &App, preferences: &ModifyPreferences) {
    let pass_key = non_empty_string(&preferences.pass_key_text)
        .unwrap_or_else(|| ModifyPreferences::default().pass_key_text);
    load_modify_pass_into_app(app, &pass_key);

    if let Some(last_valid) = preferences.last_valid_config_text.clone() {
        app.set_modify_last_valid_config_text(last_valid.into());
    }

    if let Some(config_text) = preferences
        .config_text
        .clone()
        .or_else(|| preferences.last_valid_config_text.clone())
    {
        app.set_modify_config_text(config_text.into());
    }

    validate_modify_config(app);
}

fn restore_merge_preferences(app: &App, preferences: &MergePreferences) {
    merge_text_bindings!(restore_text_fields, app, preferences);
}

fn restore_merge_session(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    session: &super::schema::UiSessionRestore,
) {
    let merge_sources = session
        .merge_sources
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    if !merge_sources.is_empty() {
        append_merge_source_paths(app, bridge, shared_state, merge_sources);
        apply_merge_sources_to_app(app, shared_state);
    }

    if let Some(path) = &session.merge_output_path {
        app.set_merge_output_path_text(path.display().to_string().into());
        app.set_merge_result_output_text(file_name_or_path(path).into());
    }
}

fn restore_selected_midi_session(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    config: &UiConfigFile,
) {
    if let Some(path) = &config.session.render_output_path {
        app.set_render_output_path_text(path.display().to_string().into());
    }
    if let Some(path) = &config.session.modify_output_path {
        app.set_modify_output_path_text(path.display().to_string().into());
        app.set_modify_result_output_text(file_name_or_path(path).into());
    }

    let restore_time = config.session.current_time.max(0.0);
    if restore_time <= 0.0 {
        return;
    }

    if let Ok(events) = bridge.seek_time(restore_time, shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
}

fn session_matches_current_midi(
    shared_state: &Arc<Mutex<UiViewModel>>,
    persisted_path: Option<&PathBuf>,
) -> bool {
    let Some(persisted_path) = persisted_path.filter(|path| path.exists()) else {
        return false;
    };

    shared_state
        .lock()
        .expect("ui model mutex poisoned")
        .scene
        .midi_path
        .as_ref()
        .is_some_and(|current_path| current_path == persisted_path)
}

fn capture_ui_config(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    seed_config: Option<&UiConfigFile>,
) -> UiConfigFile {
    let (snapshot, merge_sources) = {
        let model = shared_state.lock().expect("ui model mutex poisoned");
        (
            model.snapshot.clone(),
            model
                .merge
                .sources
                .iter()
                .map(|source| source.path.clone())
                .collect::<Vec<_>>(),
        )
    };

    let mut config = seed_config.cloned().unwrap_or_default();
    config.version = super::schema::CONFIG_VERSION;

    if let Some(snapshot) = snapshot {
        config.preferences.audio = snapshot.audio;
        config.preferences.scene = snapshot.scene;
        config.preferences.view_range = snapshot.view_range;
        config.preferences.time_space = snapshot.time_space;
        config.preferences.first_key = snapshot.first_key;
        config.preferences.last_key = snapshot.last_key;
        config.session.current_time = snapshot.current_time.max(0.0);
        config.session.midi_path = snapshot.midi_path;
    } else {
        config.session.midi_path = non_empty_path(app.get_selected_midi_name().as_str());
        config.session.current_time = 0.0;
    }

    config.preferences.active_profile = app.get_active_profile();
    config.preferences.export = capture_export_preferences(app);
    config.preferences.modify = capture_modify_preferences(app);
    config.preferences.merge = capture_merge_preferences(app);
    config.preferences.window = capture_window_preferences(app);
    config.preferences.last_palette_png =
        active_palette_path_from_scene(&config.preferences.scene).or_else(last_palette_png);
    config.preferences.last_background_png =
        active_background_path_from_scene(&config.preferences.scene).or_else(last_background_png);
    config.preferences.last_aura_png =
        active_aura_path_from_scene(&config.preferences.scene).or_else(last_aura_png);

    config.session.render_output_path = non_empty_path(app.get_render_output_path_text().as_str());
    config.session.modify_output_path = non_empty_path(app.get_modify_output_path_text().as_str());
    config.session.merge_output_path = non_empty_path(app.get_merge_output_path_text().as_str());
    config.session.merge_sources = merge_sources
        .into_iter()
        .filter(|path| !path.as_os_str().is_empty())
        .collect::<Vec<_>>();

    config
}

fn capture_export_preferences(app: &App) -> ExportPreferences {
    let mut preferences = ExportPreferences::default();
    export_text_bindings!(capture_text_fields, app, &mut preferences);
    export_bool_bindings!(capture_bool_fields, app, &mut preferences);
    preferences
}

fn capture_modify_preferences(app: &App) -> ModifyPreferences {
    ModifyPreferences {
        pass_key_text: app.get_modify_pass_key_text().to_string(),
        config_text: non_empty_string(app.get_modify_config_text().as_str()),
        last_valid_config_text: non_empty_string(app.get_modify_last_valid_config_text().as_str()),
    }
}

fn capture_merge_preferences(app: &App) -> MergePreferences {
    let mut preferences = MergePreferences::default();
    merge_text_bindings!(capture_text_fields, app, &mut preferences);
    preferences
}

fn capture_window_preferences(app: &App) -> WindowPreferences {
    let position = app.window().position();
    let size = app.window().size();

    WindowPreferences {
        position: Some(WindowPosition {
            x: position.x,
            y: position.y,
        }),
        size: Some(WindowSize {
            width: size.width.max(1),
            height: size.height.max(1),
        }),
        maximized: app.window().is_maximized(),
        fullscreen: app.window().is_fullscreen(),
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn non_empty_path(value: &str) -> Option<PathBuf> {
    non_empty_string(value).map(PathBuf::from)
}
