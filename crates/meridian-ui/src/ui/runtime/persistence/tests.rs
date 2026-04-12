use std::{fs, path::PathBuf};

use meridian_core::render::{NoteProjectorConfig, RendererKind, SceneConfig};
use tempfile::tempdir;

use super::build_startup_options;
use super::store::{
    CONFIG_BAK_FILE_NAME, CONFIG_FILE_NAME, load_ui_config_from_dir, save_ui_config_to_dir,
};
use crate::ui::runtime::persistence::schema::{
    ExportPreferences, MergePreferences, ModifyPreferences, UiConfigFile, UiPreferences,
    WindowPreferences,
};
use crate::ui::state::UiOptions;

#[test]
fn save_and_load_round_trip() {
    let dir = tempdir().expect("tempdir");
    let config = sample_config();

    save_ui_config_to_dir(Some(dir.path()), &config).expect("save config");
    let loaded = load_ui_config_from_dir(Some(dir.path()))
        .expect("load config")
        .expect("config should exist");

    assert_eq!(loaded, config);
}

#[test]
fn load_falls_back_to_backup_when_primary_is_invalid() {
    let dir = tempdir().expect("tempdir");
    let config = sample_config();

    save_ui_config_to_dir(Some(dir.path()), &config).expect("save config");
    let primary = dir.path().join(CONFIG_FILE_NAME);
    let backup = dir.path().join(CONFIG_BAK_FILE_NAME);
    fs::copy(&primary, &backup).expect("copy backup");
    fs::write(&primary, b"{ definitely not valid json").expect("corrupt primary");

    let loaded = load_ui_config_from_dir(Some(dir.path()))
        .expect("load config")
        .expect("config should exist");

    assert_eq!(loaded, config);
}

#[test]
fn startup_options_merge_preferences_and_cli_overrides() {
    let mut config = sample_config();
    config.preferences.view_range = 9.0;
    config.preferences.first_key = 12;
    config.preferences.last_key = 90;

    let launch = UiOptions {
        renderer: Some(RendererKind::Flat),
        start_time: Some(12.5),
        view_range: Some(2.5),
        first_key: Some(20),
        last_key: Some(80),
        ..UiOptions::default()
    };

    let startup = build_startup_options(&launch, Some(&config));

    assert_eq!(startup.start_time, 12.5);
    assert_eq!(startup.view_range, 2.5);
    assert_eq!(startup.first_key, 20);
    assert_eq!(startup.last_key, 80);
    assert!(matches!(
        startup.scene,
        SceneConfig::TwoD(ref two_d) if matches!(two_d.notes, NoteProjectorConfig::Flat(_))
    ));
}

fn sample_config() -> UiConfigFile {
    UiConfigFile {
        version: 1,
        preferences: UiPreferences {
            view_range: 1.5,
            first_key: 8,
            last_key: 100,
            active_profile: 4,
            export: ExportPreferences {
                mode_text: "audio_only".into(),
                range_mode_text: "custom".into(),
                start_time_text: "12.5".into(),
                end_time_text: "47.25".into(),
                video_resolution_text: "3840x2160".into(),
                video_fps_text: "120".into(),
                audio_format_text: "flac".into(),
                audio_sample_rate_text: "96000".into(),
                audio_channel_count_text: "mono".into(),
                video_ffmpeg_args_text: "-movflags +faststart".into(),
                audio_ffmpeg_args_text: "-q:a 2".into(),
                video_codec_text: "libx265".into(),
                video_crf_text: "20".into(),
                video_preset_text: "slow".into(),
                video_pix_fmt_text: "yuv444p".into(),
                video_rgb_mode_text: "straight".into(),
                audio_bitrate_text: "256k".into(),
                export_alpha_mask: true,
                use_limiter: false,
                open_after_export: true,
            },
            modify: ModifyPreferences {
                pass_key_text: "humanize".into(),
                config_text: Some("{\"tool\":{\"Quantize\":{\"rounding_ticks\":60}}}".into()),
                last_valid_config_text: Some(
                    "{\"tool\":{\"Quantize\":{\"rounding_ticks\":120}}}".into(),
                ),
            },
            merge: MergePreferences {
                layout_text: "merge_tracks".into(),
                metadata_mode_text: "normalize".into(),
                ppq_override_text: "960".into(),
            },
            window: WindowPreferences::default(),
            last_palette_png: Some(PathBuf::from("/tmp/palette.png")),
            last_background_png: Some("/tmp/background.png".into()),
            last_aura_png: Some("/tmp/aura.png".into()),
            ..UiPreferences::default()
        },
    }
}
