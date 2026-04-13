//! Render/export wiring for the UI.
//!
//! This file owns export defaults, draft construction, and top-level runtime
//! wiring. The active export lifecycle is split under `render_export/*` so the
//! UI has one place for configuration and another for in-flight job handling.

use super::*;
use meridian_core::{
    audio::AudioRenderConfig,
    protocol::{
        AudioOutputFormat, FrameColorMode, VideoAudioConfig, VideoExportConfig,
        VideoOutputContainer,
    },
    transport::PREVIEW_START_TIME_SECONDS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenderExportMode {
    VideoAudio,
    VideoOnly,
    AudioOnly,
}

impl RenderExportMode {
    pub(super) fn from_text(value: &str) -> Self {
        match value {
            "video_only" => Self::VideoOnly,
            "audio_only" => Self::AudioOnly,
            _ => Self::VideoAudio,
        }
    }

    pub(super) fn wants_video(self) -> bool {
        matches!(self, Self::VideoAudio | Self::VideoOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenderRangeMode {
    FullSong,
    Custom,
}

impl RenderRangeMode {
    pub(super) fn from_text(value: &str) -> Self {
        match value {
            "custom" => Self::Custom,
            _ => Self::FullSong,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AudioOnlyFormat {
    Wav,
    Flac,
    Mp3,
}

impl AudioOnlyFormat {
    pub(super) fn from_text(value: &str) -> Self {
        match value {
            "flac" => Self::Flac,
            "mp3" => Self::Mp3,
            _ => Self::Wav,
        }
    }

    pub(super) fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
        }
    }
}

pub(super) fn video_output_container_from_text(value: &str) -> VideoOutputContainer {
    match value {
        "mkv" => VideoOutputContainer::Mkv,
        _ => VideoOutputContainer::Mp4,
    }
}

pub fn run_ui(options: UiOptions) -> Result<(), MeridianError> {
    let persisted_config = load_ui_config();
    let startup = build_startup_options(&options, persisted_config.as_ref());
    let backend_selector = slint::BackendSelector::new();
    if startup.disable_wgpu {
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
    apply_persisted_window_preferences(&app, persisted_config.as_ref());
    let bridge = UiCoreBridge::new(spawn_core());
    let shared_state = Arc::new(Mutex::new(UiViewModel::default()));
    let preview_load_generation = Arc::new(AtomicU64::new(0));
    let render_load_generation = Arc::new(AtomicU64::new(0));
    let audio_load_generation = Arc::new(AtomicU64::new(0));
    let analysis_load_generation = Arc::new(AtomicU64::new(0));
    let export_state = install_render_export_runtime(&app, &bridge, &shared_state);
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&bridge, &startup, &app, &shared_state)?;
    initialize_merge_panel(&app, &shared_state);
    initialize_modify_panel(&app, &bridge, &shared_state);
    restore_persisted_ui_state(&app, &bridge, &shared_state, persisted_config.as_ref());
    install_midi_process_listener(&app, bridge.core(), &shared_state);
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
        startup.disable_wgpu,
        &export_state,
    );
    let _config_persistence_timer =
        install_config_persistence_timer(&app, &shared_state, persisted_config.clone());

    app.window().request_redraw();
    let run_result = app
        .run()
        .map_err(|e| MeridianError::Platform(e.to_string()));
    let _ = save_ui_config_now(&app, &shared_state, persisted_config.as_ref());
    run_result
}

pub(crate) fn initialize_core(
    bridge: &UiCoreBridge,
    options: &UiStartupOptions,
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
    wire_merge_callbacks(app, bridge, shared_state);
    wire_modify_callbacks(app, bridge, shared_state);
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

pub(super) fn set_default_render_output_path(app: &App) {
    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let video_container =
        video_output_container_from_text(app.get_render_video_container_text().as_str());
    app.set_render_output_path_text(
        default_render_output_path(
            app.get_selected_midi_name().as_str(),
            mode,
            audio_format,
            video_container,
        )
        .into(),
    );
}

pub(super) fn default_render_output_path(
    selected_midi_name: &str,
    mode: RenderExportMode,
    audio_format: AudioOnlyFormat,
    video_container: VideoOutputContainer,
) -> String {
    if selected_midi_name.is_empty() {
        return String::new();
    }

    let selected = PathBuf::from(selected_midi_name);
    let stem = selected
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("render");
    let file_name = if mode == RenderExportMode::AudioOnly {
        format!("{stem}.rendered.{}", audio_format.extension())
    } else {
        format!("{stem}.rendered.{}", video_container.extension())
    };
    selected
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
        .display()
        .to_string()
}

pub(super) fn resolve_render_output_path_text(
    current: &str,
    selected_midi_name: &str,
    mode: RenderExportMode,
    audio_format: AudioOnlyFormat,
    video_container: VideoOutputContainer,
) -> String {
    if current.is_empty() {
        default_render_output_path(selected_midi_name, mode, audio_format, video_container)
    } else {
        current.to_string()
    }
}

pub(super) fn sync_render_output_path(app: &App) {
    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let video_container =
        video_output_container_from_text(app.get_render_video_container_text().as_str());
    let current = app.get_render_output_path_text();
    let next = resolve_render_output_path_text(
        current.as_str(),
        app.get_selected_midi_name().as_str(),
        mode,
        audio_format,
        video_container,
    );
    if next != current.as_str() {
        app.set_render_output_path_text(next.into());
    }
}

/// Split a string on whitespace into individual arguments.
/// Respects basic quoting (double-quotes only) so users can
/// pass args like `-metadata title="My Song"`.
pub(super) fn shell_words(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return Vec::new();
    }
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in input.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub(super) fn parse_render_resolution(text: &str) -> Result<(u32, u32), String> {
    let Some((width, height)) = text.split_once('x') else {
        return Err(format!("invalid resolution `{text}`"));
    };
    let width = width
        .parse::<u32>()
        .map_err(|_| format!("invalid width in `{text}`"))?;
    let height = height
        .parse::<u32>()
        .map_err(|_| format!("invalid height in `{text}`"))?;
    if width == 0 || height == 0 {
        return Err("resolution must be non-zero".into());
    }
    Ok((width, height))
}

pub(super) fn parse_render_fps(text: &str) -> Result<f64, String> {
    let fps = text
        .parse::<f64>()
        .map_err(|_| format!("invalid fps `{text}`"))?;
    if fps <= 0.0 {
        return Err("fps must be > 0".into());
    }
    Ok(fps)
}

pub(super) fn parse_render_time_seconds(text: &str, label: &str) -> Result<f64, String> {
    let seconds = text
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid {label} `{text}`"))?;
    if !seconds.is_finite() {
        return Err(format!("{label} must be a finite value"));
    }
    Ok(seconds)
}

pub(crate) fn default_render_time_range_text(midi_length: f64) -> (String, String) {
    (
        PREVIEW_START_TIME_SECONDS.to_string(),
        midi_length.max(0.0).to_string(),
    )
}

pub(crate) fn apply_default_render_time_range(app: &App, midi_length: f64) {
    let (start_time, end_time) = default_render_time_range_text(midi_length);
    app.set_render_start_time_text(start_time.into());
    app.set_render_end_time_text(end_time.into());
}

pub(super) fn parse_render_channels(text: &str) -> Result<u16, String> {
    match text {
        "mono" => Ok(1),
        "stereo" => Ok(2),
        other => Err(format!("unsupported channel count `{other}`")),
    }
}

pub(super) fn current_export_output_path(app: &App) -> Result<PathBuf, String> {
    let raw = app.get_render_output_path_text();
    if raw.is_empty() {
        return Err("choose an output path".into());
    }
    Ok(PathBuf::from(raw.as_str()))
}

pub(super) fn current_custom_video_time_range(
    app: &App,
    snapshot: &StateSnapshot,
) -> Result<(Option<f64>, Option<f64>), String> {
    if RenderRangeMode::from_text(app.get_render_range_mode_text().as_str())
        != RenderRangeMode::Custom
    {
        return Ok((None, None));
    }

    let start_time = parse_render_time_seconds(
        app.get_render_start_time_text().as_str(),
        "render start time",
    )?;
    let end_time = if app.get_render_end_time_text().trim().is_empty() {
        snapshot.midi_length
    } else {
        parse_render_time_seconds(app.get_render_end_time_text().as_str(), "render end time")?
    };
    if end_time <= start_time {
        return Err("render end time must be greater than start time".into());
    }
    if end_time > snapshot.midi_length {
        return Err(format!(
            "render end time {:.3} exceeds midi length {:.3}",
            end_time, snapshot.midi_length
        ));
    }

    Ok((Some(start_time), Some(end_time)))
}

pub(super) fn build_video_render_config(
    app: &App,
    snapshot: &StateSnapshot,
    output: PathBuf,
    mode: RenderExportMode,
    container: VideoOutputContainer,
) -> Result<VideoRenderConfig, String> {
    let (width, height) = parse_render_resolution(app.get_render_video_resolution_text().as_str())?;
    let fps = parse_render_fps(app.get_render_video_fps_text().as_str())?;
    let (start_time, end_time) = current_custom_video_time_range(app, snapshot)?;

    // Build ffmpeg args from the structured encoding controls
    let mut ffmpeg_args: Vec<String> = vec!["-y".into()];
    let codec = app.get_render_video_codec_text();
    if !codec.is_empty() {
        ffmpeg_args.extend(["-c:v".into(), codec.to_string()]);
    }
    let pix_fmt = app.get_render_video_pix_fmt_text();
    if !pix_fmt.is_empty() {
        ffmpeg_args.extend(["-pix_fmt".into(), pix_fmt.to_string()]);
    }
    let crf = app.get_render_video_crf_text();
    if !crf.is_empty() {
        ffmpeg_args.extend(["-crf".into(), crf.to_string()]);
    }
    let preset = app.get_render_video_preset_text();
    if !preset.is_empty() {
        ffmpeg_args.extend(["-preset".into(), preset.to_string()]);
    }

    // Append any extra raw flags from the advanced text field
    let extra = app.get_render_video_ffmpeg_args_text();
    if !extra.is_empty() {
        ffmpeg_args.extend(shell_words(extra.as_str()));
    }
    let color_mode = match app.get_render_video_rgb_mode_text().as_str() {
        "straight" => FrameColorMode::Straight,
        _ => FrameColorMode::Premultiplied,
    };
    let audio = if mode == RenderExportMode::VideoAudio {
        let sample_rate = app
            .get_render_audio_sample_rate_text()
            .parse::<u32>()
            .map_err(|_| "invalid render audio sample rate".to_string())?;
        let channels = parse_render_channels(app.get_render_audio_channel_count_text().as_str())?;
        Some(VideoAudioConfig {
            sample_rate: Some(sample_rate),
            channels: Some(channels),
            use_limiter: Some(app.get_render_use_limiter()),
            soundfonts: Vec::new(),
            ffmpeg_args: build_render_audio_ffmpeg_args(app),
        })
    } else {
        None
    };
    Ok(VideoRenderConfig {
        midi_path: snapshot.midi_path.clone(),
        output,
        container,
        fps,
        width,
        height,
        scene: Some(snapshot.scene.clone()),
        view_range: Some(snapshot.view_range),
        time_space: Some(snapshot.time_space),
        start_time,
        end_time,
        first_key: Some(snapshot.first_key),
        last_key: Some(snapshot.last_key),
        ffmpeg_args,
        export: VideoExportConfig {
            color_mode,
            export_alpha_mask: app.get_render_export_alpha_mask(),
        },
        audio,
    })
}

pub(super) fn build_audio_render_config(
    app: &App,
    snapshot: &StateSnapshot,
    output: PathBuf,
    format: AudioOnlyFormat,
) -> Result<AudioRenderConfig, String> {
    let sample_rate = app
        .get_render_audio_sample_rate_text()
        .parse::<u32>()
        .map_err(|_| "invalid render audio sample rate".to_string())?;
    let channels = parse_render_channels(app.get_render_audio_channel_count_text().as_str())?;
    let use_limiter = app.get_render_use_limiter();
    Ok(AudioRenderConfig {
        midi_path: snapshot.midi_path.clone(),
        audio: None,
        output,
        sample_rate: Some(sample_rate),
        channels: Some(channels),
        use_limiter: Some(use_limiter),
        format: match format {
            AudioOnlyFormat::Wav => AudioOutputFormat::Wav,
            AudioOnlyFormat::Flac => AudioOutputFormat::Flac,
            AudioOnlyFormat::Mp3 => AudioOutputFormat::Mp3,
        },
        ffmpeg_args: build_render_audio_ffmpeg_args(app),
        soundfonts: Vec::new(),
    })
}

pub(super) fn build_render_audio_ffmpeg_args(app: &App) -> Vec<String> {
    let mut args = Vec::new();
    let bitrate = app.get_render_audio_bitrate_text();
    if !bitrate.is_empty() {
        args.extend(["-b:a".to_string(), bitrate.to_string()]);
    }
    args.extend(shell_words(
        app.get_render_audio_ffmpeg_args_text().as_str(),
    ));
    args
}

pub(super) fn set_export_status(
    app: &App,
    status: &str,
    detail: impl Into<slint::SharedString>,
    progress: f32,
) {
    app.set_render_export_status(status.into());
    app.set_render_export_detail_text(detail.into());
    app.set_render_export_progress(progress);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_video_output_path_is_next_to_selected_midi() {
        let path = default_render_output_path(
            "/tmp/example/song.mid",
            RenderExportMode::VideoAudio,
            AudioOnlyFormat::Wav,
            VideoOutputContainer::Mp4,
        );

        assert_eq!(path, "/tmp/example/song.rendered.mp4");
    }

    #[test]
    fn default_mkv_output_path_uses_selected_container_extension() {
        let path = default_render_output_path(
            "/tmp/example/song.mid",
            RenderExportMode::VideoOnly,
            AudioOnlyFormat::Wav,
            VideoOutputContainer::Mkv,
        );

        assert_eq!(path, "/tmp/example/song.rendered.mkv");
    }

    #[test]
    fn default_audio_output_path_uses_rendered_suffix() {
        let path = default_render_output_path(
            "/tmp/example/song.mid",
            RenderExportMode::AudioOnly,
            AudioOnlyFormat::Flac,
            VideoOutputContainer::Mp4,
        );

        assert_eq!(path, "/tmp/example/song.rendered.flac");
    }

    #[test]
    fn resolve_render_output_path_text_preserves_non_empty_paths() {
        let path = resolve_render_output_path_text(
            "/tmp/example/custom-name.txt",
            "/tmp/example/song.mid",
            RenderExportMode::AudioOnly,
            AudioOnlyFormat::Flac,
            VideoOutputContainer::Mp4,
        );

        assert_eq!(path, "/tmp/example/custom-name.txt");
    }

    #[test]
    fn resolve_render_output_path_text_defaults_only_when_empty() {
        let path = resolve_render_output_path_text(
            "",
            "/tmp/example/song.mid",
            RenderExportMode::VideoAudio,
            AudioOnlyFormat::Wav,
            VideoOutputContainer::Mkv,
        );

        assert_eq!(path, "/tmp/example/song.rendered.mkv");
    }

    #[test]
    fn parse_render_time_seconds_allows_negative_values() {
        assert_eq!(
            parse_render_time_seconds("-1", "render start time").unwrap(),
            -1.0
        );
    }

    #[test]
    fn default_render_time_range_text_uses_preview_preroll_and_song_end() {
        assert_eq!(
            default_render_time_range_text(12.5),
            ("-1".into(), "12.5".into())
        );
    }
}
