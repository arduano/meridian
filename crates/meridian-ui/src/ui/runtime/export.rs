use super::*;

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

    pub(super) fn wants_audio(self) -> bool {
        matches!(self, Self::VideoAudio | Self::AudioOnly)
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

#[derive(Debug, Clone)]
pub(super) enum RenderJobOutcome {
    Finished,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone)]
pub(super) enum FinalizeSpec {
    MuxMp4 {
        video_input: PathBuf,
        audio_input: PathBuf,
        extra_args: Vec<String>,
    },
    EncodeAudio {
        wav_input: PathBuf,
        format: AudioOnlyFormat,
        extra_args: Vec<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub(super) struct RenderExportCoordinator {
    pub(super) active: bool,
    pub(super) mode: Option<RenderExportMode>,
    pub(super) final_output: Option<PathBuf>,
    pub(super) video_job_output: Option<PathBuf>,
    pub(super) audio_job_output: Option<PathBuf>,
    pub(super) video_outcome: Option<RenderJobOutcome>,
    pub(super) audio_outcome: Option<RenderJobOutcome>,
    pub(super) finalize_spec: Option<FinalizeSpec>,
    pub(super) finalizing: bool,
    pub(super) finalize_result: Option<Result<(), String>>,
}

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
    let export_state = Arc::new(Mutex::new(RenderExportCoordinator::default()));
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&bridge, &options, &app, &shared_state)?;
    initialize_merge_panel(&app, &shared_state);
    initialize_modify_panel(&app, &bridge, &shared_state);
    install_core_event_listener(&app, bridge.core(), &shared_state, &export_state);
    install_midi_process_listener(&app, bridge.core(), &shared_state);
    wire_callbacks(
        &app,
        &bridge,
        &shared_state,
        &preview_load_generation,
        &render_load_generation,
        &audio_load_generation,
        &analysis_load_generation,
        &export_state,
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
        &export_state,
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
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) {
    wire_transport_callbacks(app, bridge, shared_state);
    wire_video_callbacks(app, bridge, shared_state);
    wire_audio_config_callbacks(app, bridge, shared_state);
    wire_render_export_callbacks(app, bridge, shared_state, export_state);
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
    app.set_render_output_path_text(
        default_render_output_path(app.get_selected_midi_name().as_str(), mode, audio_format)
            .into(),
    );
}

pub(super) fn default_render_output_path(
    selected_midi_name: &str,
    mode: RenderExportMode,
    audio_format: AudioOnlyFormat,
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
    let ext = if mode == RenderExportMode::AudioOnly {
        audio_format.extension()
    } else {
        "mp4"
    };
    format!("{stem}.{ext}")
}

pub(super) fn normalize_output_path(
    path: &Path,
    mode: RenderExportMode,
    audio_format: AudioOnlyFormat,
) -> PathBuf {
    let extension = if mode == RenderExportMode::AudioOnly {
        audio_format.extension()
    } else {
        "mp4"
    };
    let mut normalized = path.to_path_buf();
    normalized.set_extension(extension);
    normalized
}

pub(super) fn sync_render_output_path(app: &App) {
    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let current = app.get_render_output_path_text();
    let next = if current.is_empty() {
        default_render_output_path(app.get_selected_midi_name().as_str(), mode, audio_format)
    } else {
        normalize_output_path(Path::new(current.as_str()), mode, audio_format)
            .display()
            .to_string()
    };
    app.set_render_output_path_text(next.into());
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

pub(super) fn parse_render_channels(text: &str) -> Result<u16, String> {
    match text {
        "mono" => Ok(1),
        "stereo" => Ok(2),
        other => Err(format!("unsupported channel count `{other}`")),
    }
}

pub(super) fn next_export_temp_path(final_output: &Path, label: &str, extension: &str) -> PathBuf {
    let id = EXPORT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let stem = final_output
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("render");
    let dir = final_output
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join(format!(".{stem}.meridian-{id}.{label}.{extension}"))
}

pub(super) fn current_export_output_path(app: &App) -> Result<PathBuf, String> {
    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let raw = if app.get_render_output_path_text().is_empty() {
        default_render_output_path(app.get_selected_midi_name().as_str(), mode, audio_format)
    } else {
        app.get_render_output_path_text().to_string()
    };
    if raw.is_empty() {
        return Err("select a MIDI before exporting".into());
    }
    Ok(normalize_output_path(Path::new(&raw), mode, audio_format))
}

pub(super) fn build_video_render_config(
    app: &App,
    snapshot: &StateSnapshot,
    output: PathBuf,
) -> Result<VideoRenderConfig, String> {
    let (width, height) = parse_render_resolution(app.get_render_video_resolution_text().as_str())?;
    let fps = parse_render_fps(app.get_render_video_fps_text().as_str())?;

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
    Ok(VideoRenderConfig {
        midi_path: snapshot.midi_path.clone(),
        output,
        fps,
        width,
        height,
        scene: Some(snapshot.scene.clone()),
        view_range: Some(snapshot.view_range),
        time_space: Some(snapshot.time_space),
        first_key: Some(snapshot.first_key),
        last_key: Some(snapshot.last_key),
        ffmpeg_args,
        export: Default::default(),
    })
}

pub(super) fn build_audio_render_config(
    app: &App,
    snapshot: &StateSnapshot,
    output: PathBuf,
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
        soundfonts: Vec::new(),
    })
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

fn run_ffmpeg(args: &[&str]) -> Result<(), String> {
    let status = Command::new("ffmpeg")
        .args(args)
        .status()
        .map_err(|error| format!("failed to launch ffmpeg: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg exited with status {status}"))
    }
}

pub(super) fn run_finalizer(spec: &FinalizeSpec, final_output: &Path) -> Result<(), String> {
    match spec {
        FinalizeSpec::MuxMp4 {
            video_input,
            audio_input,
            extra_args,
        } => {
            let mut args = vec![
                "-y".to_string(),
                "-i".into(),
                video_input.to_string_lossy().into_owned(),
                "-i".into(),
                audio_input.to_string_lossy().into_owned(),
                "-c:v".into(),
                "copy".into(),
                "-c:a".into(),
                "aac".into(),
            ];
            args.extend(extra_args.iter().cloned());
            args.push(final_output.to_string_lossy().into_owned());
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_ffmpeg(&arg_refs)
        }
        FinalizeSpec::EncodeAudio {
            wav_input,
            format,
            extra_args,
        } => match format {
            AudioOnlyFormat::Wav => fs::copy(wav_input, final_output)
                .map(|_| ())
                .map_err(|error| format!("failed to copy wav output: {error}")),
            AudioOnlyFormat::Flac => {
                let mut args = vec![
                    "-y".to_string(),
                    "-i".into(),
                    wav_input.to_string_lossy().into_owned(),
                ];
                args.extend(extra_args.iter().cloned());
                args.push(final_output.to_string_lossy().into_owned());
                let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                run_ffmpeg(&arg_refs)
            }
            AudioOnlyFormat::Mp3 => {
                let mut args = vec![
                    "-y".to_string(),
                    "-i".into(),
                    wav_input.to_string_lossy().into_owned(),
                    "-codec:a".into(),
                    "libmp3lame".into(),
                    "-q:a".into(),
                    "2".into(),
                ];
                args.extend(extra_args.iter().cloned());
                args.push(final_output.to_string_lossy().into_owned());
                let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                run_ffmpeg(&arg_refs)
            }
        },
    }
}
