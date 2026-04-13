use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::ComponentHandle;

use super::{
    App, Arc, Mutex, UiCoreBridge, UiViewModel, load_modify_pass_into_app, validate_modify_config,
};

use super::{
    assets::{
        active_aura_path_from_scene, active_background_path_from_scene,
        active_palette_path_from_scene, restore_last_asset_paths,
    },
    schema::{
        ExportPreferences, MergePreferences, ModifyPreferences, UiConfigFile, WindowPreferences,
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
            range_mode_text => (get_render_range_mode_text, set_render_range_mode_text),
            start_time_text => (get_render_start_time_text, set_render_start_time_text),
            end_time_text => (get_render_end_time_text, set_render_end_time_text),
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
    _bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    config: Option<&UiConfigFile>,
) {
    let Some(config) = config else {
        return;
    };

    app.set_active_profile(config.preferences.active_profile);
    {
        let mut state = shared_state.lock().expect("ui model mutex poisoned");
        restore_last_asset_paths(&mut state, &config.preferences);
    }
    let midi_length = shared_state
        .lock()
        .expect("ui model mutex poisoned")
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.midi_length);
    restore_export_preferences(app, &config.preferences.export, midi_length);
    restore_modify_preferences(app, &config.preferences.modify);
    restore_merge_preferences(app, &config.preferences.merge);
}

fn restore_export_preferences(
    app: &App,
    preferences: &ExportPreferences,
    midi_length: Option<f64>,
) {
    let preferences = restored_export_preferences(preferences, midi_length);
    export_text_bindings!(restore_text_fields, app, preferences);
    export_bool_bindings!(restore_bool_fields, app, preferences);
}

pub(in super::super) fn restored_export_preferences(
    preferences: &ExportPreferences,
    midi_length: Option<f64>,
) -> ExportPreferences {
    let mut restored = preferences.clone();
    if let Some(midi_length) = midi_length {
        let (start_time, end_time) =
            super::super::export::default_render_time_range_text(midi_length);
        restored.start_time_text = start_time;
        restored.end_time_text = end_time;
    }
    restored
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

fn capture_ui_config(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    seed_config: Option<&UiConfigFile>,
) -> UiConfigFile {
    let (snapshot, remembered_assets) = {
        let model = shared_state.lock().expect("ui model mutex poisoned");
        (model.snapshot.clone(), model.remembered_assets.clone())
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
    }

    config.preferences.active_profile = app.get_active_profile();
    config.preferences.export = capture_export_preferences(app);
    config.preferences.modify = capture_modify_preferences(app);
    config.preferences.merge = capture_merge_preferences(app);
    config.preferences.window = capture_window_preferences(app);
    config.preferences.last_palette_png = active_palette_path_from_scene(&config.preferences.scene)
        .or_else(|| remembered_assets.palette_png.clone());
    config.preferences.last_background_png =
        active_background_path_from_scene(&config.preferences.scene)
            .or_else(|| remembered_assets.background_png.clone());
    config.preferences.last_aura_png = active_aura_path_from_scene(&config.preferences.scene)
        .or_else(|| remembered_assets.aura_png.clone());

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
    let _ = app;
    WindowPreferences::default()
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
