use std::{
    cell::RefCell,
    ffi::OsStr,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use meridian_core::{
    CoreHandle, MeridianError,
    audio::{
        AudioConfig, AudioRenderConfig, ChannelCount, DEFAULT_SOUNDFONT, EnvelopeCurveType,
        Interpolator, MeridianSoundfont, ThreadCount,
    },
    display::MIN_VIEW_RANGE_SECONDS,
    midi::{
        AnalysisGuardTool, ChannelMapEntry, ChannelProgram, ChannelRemapTool, ControlChangeTool,
        ControlValue, ControllerMapEntry, ControllerScaleEntry, DedupeTool, HumanizeTool,
        KeyMapEntry, KeyMapTool, KeyRange, MergeBalanceTool, MetaTextTool,
        MidiFileProcessingConfig, MidiFileSelection, MidiModifierTool, MidiProcessingConfig,
        NoteLengthTool, OverlapRepairTool, PitchBendTool, ProgramTool, QuantizeTool,
        RangeSelectTool, SelectableEventKind, SharedMetadataTrackTool, SysexTool, TempoMapTool,
        TempoPoint, TextKind, TimeWarpPoint, TimeWarpTool, TrackMapEntry, TrackRouteTool,
        TrimProcessingConfig, VelocityMapTool, VelocityPoint,
    },
    protocol::{CoreEvent, ParsedMidiId, ProcessedMidiId, StateSnapshot, VideoRenderConfig},
    render::{
        DisplayTimeSpace, KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig,
        NoteProjectorConfig, PFA_BLUE_TOP_BAR_COLOR, PFA_GREEN_TOP_BAR_COLOR,
        PFA_RED_TOP_BAR_COLOR, PfaKeyboardProjectorConfig, PianoTrailClassicSceneConfig,
        ProjectorImageConfig, RendererKind, SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
    },
    spawn_core,
};
use serde_json::Value;
use slint::ComponentHandle;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};

use super::{
    core_bridge::UiCoreBridge,
    state::{UiOptions, apply_events_to_app, apply_merge_sources_to_app},
    view::{App, MidiLoadState},
    view_model::{MergeSourceInspection, MergeSourceViewModel, UiViewModel},
    viewport::ViewportRenderer,
};

static LAST_PALETTE_PNG: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));
static LAST_AURA_PNG: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderExportMode {
    VideoAudio,
    VideoOnly,
    AudioOnly,
}

impl RenderExportMode {
    fn from_text(value: &str) -> Self {
        match value {
            "video_only" => Self::VideoOnly,
            "audio_only" => Self::AudioOnly,
            _ => Self::VideoAudio,
        }
    }

    fn wants_video(self) -> bool {
        matches!(self, Self::VideoAudio | Self::VideoOnly)
    }

    fn wants_audio(self) -> bool {
        matches!(self, Self::VideoAudio | Self::AudioOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioOnlyFormat {
    Wav,
    Flac,
    Mp3,
}

impl AudioOnlyFormat {
    fn from_text(value: &str) -> Self {
        match value {
            "flac" => Self::Flac,
            "mp3" => Self::Mp3,
            _ => Self::Wav,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
        }
    }
}

#[derive(Debug, Clone)]
enum RenderJobOutcome {
    Finished,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone)]
enum FinalizeSpec {
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
struct RenderExportCoordinator {
    active: bool,
    mode: Option<RenderExportMode>,
    final_output: Option<PathBuf>,
    video_job_output: Option<PathBuf>,
    audio_job_output: Option<PathBuf>,
    video_outcome: Option<RenderJobOutcome>,
    audio_outcome: Option<RenderJobOutcome>,
    finalize_spec: Option<FinalizeSpec>,
    finalizing: bool,
    finalize_result: Option<Result<(), String>>,
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

fn app_is_loading(app: &App) -> bool {
    app.get_render_load_state() == MidiLoadState::Loading
        || app.get_audio_load_state() == MidiLoadState::Loading
        || app.get_analysis_load_state() == MidiLoadState::Loading
}

fn cancel_pending_midi_loads(
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    preview_load_generation.fetch_add(1, Ordering::SeqCst);
    render_load_generation.fetch_add(1, Ordering::SeqCst);
    audio_load_generation.fetch_add(1, Ordering::SeqCst);
    analysis_load_generation.fetch_add(1, Ordering::SeqCst);
}

fn reset_midi_ui_state(app: &App, state: MidiLoadState) {
    app.set_viewport_image(slint::Image::default());
    app.set_render_load_state(state);
    app.set_audio_load_state(state);
    app.set_analysis_load_state(state);
    app.set_audio_load_error(Default::default());
    app.set_render_load_error(Default::default());
    app.set_analysis_load_error(Default::default());
    app.set_audio_loading_progress(0.0);
    app.set_audio_loading_status(Default::default());
    app.set_render_loading_progress(0.0);
    app.set_render_loading_status(Default::default());
    app.set_analysis_loading_progress(0.0);
    app.set_analysis_loading_status(Default::default());
    reset_analysis_outputs(app);
}

fn unload_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, Default::default());
    reset_midi_ui_state(app, MidiLoadState::NoMidi);
    app.window().request_redraw();
}

fn replace_selected_midi(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
    selected_midi_name: slint::SharedString,
) {
    cancel_pending_midi_loads(
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
    if let Ok(events) = bridge.unload_render_context(shared_state) {
        apply_events_to_app(app, shared_state, &events);
    }
    set_selected_midi(app, selected_midi_name);
    reset_midi_ui_state(app, MidiLoadState::Selected);
    app.window().request_redraw();
}

fn update_video_scene(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    mutate: impl FnOnce(&mut SceneConfig),
) {
    if let Ok(events) = bridge.update_scene(shared_state, mutate) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

fn update_video_key_range(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    first_key: u8,
    last_key: u8,
) {
    if let Ok(events) = bridge.set_key_range(
        first_key.min(last_key),
        first_key.max(last_key),
        shared_state,
    ) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

fn update_video_view_range(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    seconds: f64,
) {
    if let Ok(events) =
        bridge.set_view_range_value(seconds.max(MIN_VIEW_RANGE_SECONDS), shared_state)
    {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

fn update_audio_config(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    mutate: impl FnOnce(&mut AudioConfig),
) {
    let Some(mut config) = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.audio.clone())
    else {
        return;
    };

    mutate(&mut config);
    if let Ok(events) = bridge.set_audio_config(config, shared_state) {
        apply_events_to_app(app, shared_state, &events);
        app.window().request_redraw();
    }
}

fn primary_soundfont(config: &mut AudioConfig) -> &mut MeridianSoundfont {
    if config.soundfonts.is_empty() {
        config.soundfonts.push(MeridianSoundfont::default());
    }
    config
        .soundfonts
        .first_mut()
        .expect("audio config must always have a primary soundfont")
}

fn set_selected_midi(app: &App, selected_midi_name: slint::SharedString) {
    app.set_selected_midi_name(selected_midi_name.clone());
    app.set_window_title(window_title_for_selected(selected_midi_name.as_str()));
    set_default_render_output_path(app);
    if selected_midi_name.is_empty() {
        app.set_modify_output_path_text(Default::default());
    } else {
        app.set_modify_output_path_text(
            default_modify_output_path(selected_midi_name.as_str())
                .display()
                .to_string()
                .into(),
        );
    }
}

fn window_title_for_selected(selected_midi_name: &str) -> slint::SharedString {
    let selected_path = PathBuf::from(selected_midi_name);
    let Some(file_name) = selected_path
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
    else {
        return "Meridian".into();
    };

    format!("Meridian - {file_name}").into()
}

fn set_default_render_output_path(app: &App) {
    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    app.set_render_output_path_text(
        default_render_output_path(app.get_selected_midi_name().as_str(), mode, audio_format)
            .into(),
    );
}

fn default_render_output_path(
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

fn normalize_output_path(
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

fn sync_render_output_path(app: &App) {
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
fn shell_words(input: &str) -> Vec<String> {
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

fn parse_render_resolution(text: &str) -> Result<(u32, u32), String> {
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

fn parse_render_fps(text: &str) -> Result<f64, String> {
    let fps = text
        .parse::<f64>()
        .map_err(|_| format!("invalid fps `{text}`"))?;
    if fps <= 0.0 {
        return Err("fps must be > 0".into());
    }
    Ok(fps)
}

fn parse_render_channels(text: &str) -> Result<u16, String> {
    match text {
        "mono" => Ok(1),
        "stereo" => Ok(2),
        other => Err(format!("unsupported channel count `{other}`")),
    }
}

fn next_export_temp_path(final_output: &Path, label: &str, extension: &str) -> PathBuf {
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

fn current_export_output_path(app: &App) -> Result<PathBuf, String> {
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

fn build_video_render_config(
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
    })
}

fn build_audio_render_config(
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

fn set_export_status(
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

fn run_finalizer(spec: &FinalizeSpec, final_output: &Path) -> Result<(), String> {
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

fn wire_transport_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_step_time(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.step_time(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.zoom(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.toggle_play(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_seek_time(move |time| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.seek_time(time as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_renderer(move |renderer| {
            if let Some(app) = app_weak.upgrade() {
                let renderer = match renderer.as_str() {
                    "flat" => RendererKind::Flat,
                    "piano_trail_classic" | "3d" => RendererKind::PianoTrailClassic,
                    _ => RendererKind::Pfa,
                };
                if let Ok(events) = bridge.set_renderer(renderer, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_time_space(move |time_space| {
            if let Some(app) = app_weak.upgrade() {
                let time_space = match time_space.as_str() {
                    "tick" => DisplayTimeSpace::Tick,
                    _ => DisplayTimeSpace::Time,
                };
                if let Ok(events) = bridge.set_time_space(time_space, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
}

fn wire_video_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
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
                                randomize: true,
                            }
                        } else {
                            NotePaletteConfig::DefaultTrackColors
                        };
                    });
                }
                "palette_kind" => {
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(NotePaletteConfig::ZenithPalette { palette, .. }) =
                            palette_mut(scene)
                        else {
                            return;
                        };
                        *palette = match value.as_str() {
                            "random_gradients" => ZenithPaletteSpec::RandomGradients,
                            "png_file" => match LAST_PALETTE_PNG
                                .lock()
                                .expect("palette png mutex poisoned")
                                .clone()
                            {
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
                    update_video_scene(&app, &bridge, &shared_state, move |scene| {
                        let Some(config) = ptc_mut(scene) else {
                            return;
                        };
                        config.aura_image = if value.as_str() == "builtin" {
                            ProjectorImageConfig::Builtin {
                                name: "ring".into(),
                            }
                        } else {
                            match LAST_AURA_PNG
                                .lock()
                                .expect("aura png mutex poisoned")
                                .clone()
                            {
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
    // ── Float-valued slider callback ──
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

fn wire_audio_config_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_sample_rate(move |rate| {
            let Ok(sample_rate) = rate.parse::<u32>() else {
                return;
            };
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.audio_params.sample_rate = sample_rate;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_channel_count(move |channels| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.audio_params.channels = if channels.as_str() == "mono" {
                        ChannelCount::Mono
                    } else {
                        ChannelCount::Stereo
                    };
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_render_window(move |window_ms| {
            let Ok(render_window_ms) = window_ms.parse::<f64>() else {
                return;
            };
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.config.render_window_ms = render_window_ms;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_soundfont(move || {
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("Soundfonts", &["sf2", "sfz", "SF2", "SFZ"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        update_audio_config(&app, &bridge, &shared_state, move |config| {
                            let soundfont = primary_soundfont(config);
                            soundfont.path = path;
                            soundfont.enabled = true;
                        });
                    });
                }
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_reset_audio_soundfont(move || {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let soundfont = primary_soundfont(config);
                    soundfont.path = PathBuf::from(DEFAULT_SOUNDFONT);
                    soundfont.enabled = true;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_audio_soundfont_enabled(move || {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let soundfont = primary_soundfont(config);
                    soundfont.enabled = !soundfont.enabled;
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_interpolation(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.interpolator = if mode.as_str() == "linear" {
                            Interpolator::Linear
                        } else {
                            Interpolator::Nearest
                        };
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_effects(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.use_effects = mode.as_str() == "on";
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_layer_limit(move |limit| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    if limit.as_str() == "off" {
                        config.xsynth.limit_layers = false;
                    } else if let Ok(layers) = limit.parse::<usize>() {
                        config.xsynth.limit_layers = true;
                        config.xsynth.layers = layers;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_threading(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.config.multithreading = match mode.as_str() {
                        "auto" => ThreadCount::Auto,
                        "4" => ThreadCount::Manual(4),
                        _ => ThreadCount::None,
                    };
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_ignore_range(move |limit| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.config.ignore_range = match limit.as_str() {
                        "8" => 1..=8,
                        "16" => 1..=16,
                        "24" => 1..=24,
                        _ => 0..=0,
                    };
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_attack_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.attack_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_decay_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.decay_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_release_curve(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    let curve = envelope_curve_from_label(mode.as_str());
                    for soundfont in &mut config.soundfonts {
                        soundfont.options.vol_envelope_options.release_curve = curve;
                    }
                });
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_audio_limiter(move |mode| {
            if let Some(app) = app_weak.upgrade() {
                update_audio_config(&app, &bridge, &shared_state, move |config| {
                    config.xsynth.render.use_limiter = mode.as_str() == "on";
                });
            }
        });
    }
}

fn initialize_merge_panel(app: &App, shared_state: &Arc<Mutex<UiViewModel>>) {
    apply_merge_sources_to_app(app, shared_state);
    if app.get_merge_output_path_text().is_empty() {
        app.set_merge_result_output_text("merged-output.mid".into());
    }
}

fn wire_merge_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_select_merge_files(move || {
            if app_weak.upgrade().is_none() {
                return;
            }
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            std::thread::spawn(move || {
                let files = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_files()
                    .unwrap_or_default();
                if files.is_empty() {
                    return;
                }
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    append_merge_source_paths(&app, &bridge, &shared_state, files);
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_clear_merge_files(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                model.merge.sources.clear();
            }
            app.set_merge_output_path_text("".into());
            app.set_merge_status_text("Ready".into());
            app.set_merge_detail_text("Drop MIDI files here or browse to assemble a merge.".into());
            app.set_merge_result_output_text("merged-output.mid".into());
            apply_merge_sources_to_app(&app, &shared_state);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_remove_merge_source(move |index| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                let index = index.max(0) as usize;
                if index < model.merge.sources.len() {
                    model.merge.sources.remove(index);
                }
            }
            apply_merge_sources_to_app(&app, &shared_state);
            if app.get_merge_output_path_text().is_empty() {
                if let Some(path) = merge_default_output_from_model(&shared_state) {
                    app.set_merge_output_path_text(path.display().to_string().into());
                    app.set_merge_result_output_text(file_name_or_path(&path).into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_move_merge_source(move |index, delta| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                let index = index.max(0) as usize;
                if index >= model.merge.sources.len() {
                    return;
                }
                let target = index as isize + delta as isize;
                if !(0..model.merge.sources.len() as isize).contains(&target) {
                    return;
                }
                model.merge.sources.swap(index, target as usize);
            }
            apply_merge_sources_to_app(&app, &shared_state);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_update_merge_control(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            update_merge_control(&app, key.as_str(), value.as_str());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_browse_merge_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let suggested_output = current_merge_output_path(&app)
                .ok()
                .or_else(|| merge_default_output_from_model(&shared_state))
                .unwrap_or_else(|| PathBuf::from("merged-output.mid"));
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new().add_filter("MIDI", &["mid", "midi"]);
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_merge_output_path(&path);
                    app.set_merge_output_path_text(normalized.display().to_string().into());
                    app.set_merge_result_output_text(file_name_or_path(&normalized).into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_start_merge_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_merge_job_active() {
                return;
            }

            let inputs = merge_inputs_from_model(&shared_state);
            if inputs.is_empty() {
                set_merge_ui_failure(&app, "add at least one MIDI file");
                app.window().request_redraw();
                return;
            }

            let output = match current_merge_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_merge_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };
            app.set_merge_output_path_text(output.display().to_string().into());

            let config = build_merge_processing_config(&app);
            let events = match bridge.start_process_midi_files(
                MidiFileSelection { inputs },
                output,
                config,
                &shared_state,
            ) {
                Ok(events) => events,
                Err(error) => {
                    set_merge_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };

            if let Some(message) = events_error_message(&events) {
                set_merge_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_cancel_merge_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let events = match bridge.cancel_process_midi_files(&shared_state) {
                Ok(events) => events,
                Err(error) => {
                    set_merge_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };
            if let Some(message) = events_error_message(&events) {
                set_merge_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
}

fn append_merge_source_paths(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    paths: Vec<PathBuf>,
) {
    let mut pending = Vec::new();
    {
        let mut model = shared_state.lock().expect("ui model mutex poisoned");
        for path in paths {
            if model.merge.sources.iter().any(|source| source.path == path) {
                continue;
            }
            model
                .merge
                .sources
                .push(MergeSourceViewModel::new_loading(path.clone()));
            pending.push(path);
        }
    }

    if pending.is_empty() {
        return;
    }

    if app.get_merge_output_path_text().is_empty() {
        if let Some(path) = merge_default_output_from_model(shared_state) {
            app.set_merge_output_path_text(path.display().to_string().into());
            app.set_merge_result_output_text(file_name_or_path(&path).into());
        }
    }
    app.set_merge_status_text("Inspecting sources".into());
    app.set_merge_detail_text(
        format!(
            "Loading {} new MIDI source{}.",
            pending.len(),
            if pending.len() == 1 { "" } else { "s" }
        )
        .into(),
    );
    apply_merge_sources_to_app(app, shared_state);
    app.window().request_redraw();

    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    std::thread::spawn(move || {
        let result = bridge.inspect_midi_files(pending.clone(), &shared_state);
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            {
                let mut model = shared_state.lock().expect("ui model mutex poisoned");
                match result {
                    Ok(inspections) => {
                        for inspection in inspections {
                            if let Some(source) = model
                                .merge
                                .sources
                                .iter_mut()
                                .find(|source| source.path == inspection.path)
                            {
                                source.inspection = if let Some(message) = inspection.error.clone()
                                {
                                    MergeSourceInspection::Error(message)
                                } else {
                                    MergeSourceInspection::Ready(inspection)
                                };
                            }
                        }
                    }
                    Err(error) => {
                        for path in pending {
                            if let Some(source) = model
                                .merge
                                .sources
                                .iter_mut()
                                .find(|source| source.path == path)
                            {
                                source.inspection = MergeSourceInspection::Error(error.to_string());
                            }
                        }
                    }
                }
            }
            apply_merge_sources_to_app(&app, &shared_state);
            app.set_merge_status_text("Ready".into());
            app.set_merge_detail_text(
                "Source queue updated. Reorder files, choose a merge recipe, then write output."
                    .into(),
            );
            app.window().request_redraw();
        });
    });
}

fn merge_inputs_from_model(shared_state: &Arc<Mutex<UiViewModel>>) -> Vec<PathBuf> {
    shared_state
        .lock()
        .expect("ui model mutex poisoned")
        .merge
        .sources
        .iter()
        .map(|source| source.path.clone())
        .collect()
}

fn merge_default_output_from_model(shared_state: &Arc<Mutex<UiViewModel>>) -> Option<PathBuf> {
    let inputs = merge_inputs_from_model(shared_state);
    (!inputs.is_empty()).then(|| default_merge_output_path(&inputs))
}

fn initialize_modify_panel(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    load_modify_pass_into_app(app, "quantize");
    if app.get_modify_output_path_text().is_empty() && !app.get_selected_midi_name().is_empty() {
        app.set_modify_output_path_text(
            default_modify_output_path(app.get_selected_midi_name().as_str())
                .display()
                .to_string()
                .into(),
        );
    }
    if let Ok(status) = bridge.get_process_midi_status(shared_state) {
        apply_events_to_app(
            app,
            shared_state,
            &[CoreEvent::MidiProcessStatus { status }],
        );
    }
}

fn wire_modify_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
    {
        let app_weak = app.as_weak();
        app.on_select_modify_pass(move |key| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            load_modify_pass_into_app(&app, key.as_str());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_update_modify_control(move |key, value| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match update_modify_control(&app, key.as_str(), value.as_str()) {
                Ok(()) => app.window().request_redraw(),
                Err(message) => {
                    app.set_modify_config_status_text(
                        format!("Input error: {message}. Config unchanged.").into(),
                    );
                    app.window().request_redraw();
                }
            }
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_edit_modify_config(move |text| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_modify_config_text(text);
            validate_modify_config(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_reset_modify_config(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let current_key = app.get_modify_pass_key_text();
            let key = match current_key.as_str() {
                "" | "custom" => "quantize",
                other => other,
            };
            load_modify_pass_into_app(&app, key);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_browse_modify_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let suggested_output = current_modify_output_path(&app).unwrap_or_else(|_| {
                default_modify_output_path(app.get_selected_midi_name().as_str())
            });
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new().add_filter("MIDI", &["mid", "midi"]);
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_modify_output_path(&path);
                    app.set_modify_output_path_text(normalized.display().to_string().into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_start_modify_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if app.get_modify_job_active() {
                return;
            }

            let selected = app.get_selected_midi_name();
            if selected.is_empty() {
                set_modify_ui_failure(&app, "choose a MIDI file first");
                app.window().request_redraw();
                return;
            }

            let output = match current_modify_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_modify_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };
            app.set_modify_output_path_text(output.display().to_string().into());

            let config = match parse_modify_config(&app) {
                Ok(config) => config,
                Err(message) => {
                    set_modify_ui_failure(&app, &message);
                    app.window().request_redraw();
                    return;
                }
            };

            let events = match bridge.start_process_midi_files(
                MidiFileSelection {
                    inputs: vec![PathBuf::from(selected.as_str())],
                },
                output,
                config,
                &shared_state,
            ) {
                Ok(events) => events,
                Err(error) => {
                    set_modify_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };

            if let Some(message) = events_error_message(&events) {
                set_modify_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_cancel_modify_job(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let events = match bridge.cancel_process_midi_files(&shared_state) {
                Ok(events) => events,
                Err(error) => {
                    set_modify_ui_failure(&app, &error.to_string());
                    app.window().request_redraw();
                    return;
                }
            };
            if let Some(message) = events_error_message(&events) {
                set_modify_ui_failure(&app, &message);
            } else {
                apply_events_to_app(&app, &shared_state, &events);
            }
            app.window().request_redraw();
        });
    }
}

fn selected_modify_pass_key(app: &App) -> String {
    match app.get_modify_pass_key_text().as_str() {
        "" | "custom" => "quantize".to_string(),
        key => key.to_string(),
    }
}

fn serialize_modify_config(config: &MidiFileProcessingConfig) -> String {
    serde_json::to_string_pretty(config).unwrap_or_else(|_| "{\n  \"tools\": []\n}".into())
}

fn parse_modify_config_text(raw: &str) -> Result<MidiFileProcessingConfig, String> {
    if raw.trim().is_empty() {
        return Err("modify config is empty".into());
    }

    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("invalid JSON: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("invalid process config: {error}"))
}

fn fallback_modify_config(app: &App) -> MidiFileProcessingConfig {
    parse_modify_config_text(app.get_modify_config_text().as_str())
        .or_else(|_| parse_modify_config_text(app.get_modify_last_valid_config_text().as_str()))
        .unwrap_or_else(|_| default_modify_config(selected_modify_pass_key(app).as_str()))
}

fn set_modify_config(app: &App, config: &MidiFileProcessingConfig) {
    app.set_modify_config_text(serialize_modify_config(config).into());
    validate_modify_config(app);
}

fn option_text<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn toggle_text(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn parse_value<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Display,
{
    raw.trim()
        .parse::<T>()
        .map_err(|error| format!("invalid {label}: {error}"))
}

fn parse_optional_value<T>(raw: &str, label: &str) -> Result<Option<T>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse_value(trimmed, label).map(Some)
    }
}

fn parse_bool_toggle(raw: &str, label: &str) -> Result<bool, String> {
    match raw.trim() {
        "on" | "yes" | "true" | "1" => Ok(true),
        "off" | "no" | "false" | "0" => Ok(false),
        other => Err(format!("invalid {label}: {other}")),
    }
}

fn parse_serde_enum<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    serde_json::from_str::<T>(&format!("\"{trimmed}\""))
        .map_err(|_| format!("invalid {label}: {trimmed}"))
}

fn format_serde_enum<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"\"".into())
        .trim_matches('"')
        .to_string()
}

fn csv_parts<'a>(raw: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
}

fn parse_u8_token(raw: &str, label: &str) -> Result<u8, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|error| format!("invalid {label}: {error}"))
    } else {
        parse_value(trimmed, label)
    }
}

fn parse_arrow_entries<T>(raw: &str, label: &str) -> Result<Vec<(T, T)>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let mut result = Vec::new();
    for entry in csv_parts(raw) {
        let Some((from, to)) = entry.split_once("->") else {
            return Err(format!("invalid {label} entry: {entry}"));
        };
        result.push((
            parse_value(from.trim(), label)?,
            parse_value(to.trim(), label)?,
        ));
    }
    Ok(result)
}

fn parse_event_kinds(raw: &str) -> Result<Vec<SelectableEventKind>, String> {
    csv_parts(raw)
        .map(|entry| parse_serde_enum(entry, "event kind"))
        .collect()
}

fn format_event_kinds(kinds: &[SelectableEventKind]) -> String {
    kinds
        .iter()
        .map(format_serde_enum)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_tempo_points(raw: &str) -> Result<Vec<TempoPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((tick, tempo)) = entry.split_once(':') else {
            return Err(format!("invalid tempo point: {entry}"));
        };
        points.push(TempoPoint {
            tick: parse_value(tick.trim(), "tempo tick")?,
            tempo: parse_value(tempo.trim(), "tempo value")?,
        });
    }
    Ok(points)
}

fn format_tempo_points(points: &[TempoPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", point.tick, point.tempo))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_time_warp_points(raw: &str) -> Result<Vec<TimeWarpPoint>, String> {
    let mut points = Vec::new();
    for (source_tick, dest_tick) in parse_arrow_entries::<u64>(raw, "time-warp point")? {
        points.push(TimeWarpPoint {
            source_tick,
            dest_tick,
        });
    }
    Ok(points)
}

fn format_time_warp_points(points: &[TimeWarpPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}->{}", point.source_tick, point.dest_tick))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_channel_mappings(raw: &str) -> Result<Vec<ChannelMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "channel mapping")? {
        mappings.push(ChannelMapEntry { from, to });
    }
    Ok(mappings)
}

fn format_channel_mappings(mappings: &[ChannelMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_track_mappings(raw: &str) -> Result<Vec<TrackMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<usize>(raw, "track mapping")? {
        mappings.push(TrackMapEntry { from, to });
    }
    Ok(mappings)
}

fn format_track_mappings(mappings: &[TrackMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_channel_programs(raw: &str) -> Result<Vec<ChannelProgram>, String> {
    let mut programs = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, program)) = entry.split_once(':') else {
            return Err(format!("invalid startup program: {entry}"));
        };
        programs.push(ChannelProgram {
            channel: parse_value(channel.trim(), "startup channel")?,
            program: parse_value(program.trim(), "startup program")?,
        });
    }
    Ok(programs)
}

fn format_channel_programs(programs: &[ChannelProgram]) -> String {
    programs
        .iter()
        .map(|program| format!("{}:{}", program.channel, program.program))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_u8_list(raw: &str, label: &str) -> Result<Vec<u8>, String> {
    csv_parts(raw)
        .map(|entry| parse_value(entry, label))
        .collect()
}

fn format_u8_list(values: &[u8]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_controller_mappings(raw: &str) -> Result<Vec<ControllerMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "controller mapping")? {
        mappings.push(ControllerMapEntry { from, to });
    }
    Ok(mappings)
}

fn format_controller_mappings(mappings: &[ControllerMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_controller_scales(raw: &str) -> Result<Vec<ControllerScaleEntry>, String> {
    let mut entries = Vec::new();
    for entry in csv_parts(raw) {
        let Some((controller, scale)) = entry.split_once(':') else {
            return Err(format!("invalid controller scale: {entry}"));
        };
        entries.push(ControllerScaleEntry {
            controller: parse_value(controller.trim(), "scale controller")?,
            scale: parse_value(scale.trim(), "scale factor")?,
        });
    }
    Ok(entries)
}

fn format_controller_scales(entries: &[ControllerScaleEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.controller, entry.scale))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_control_values(raw: &str) -> Result<Vec<ControlValue>, String> {
    let mut values = Vec::new();
    for entry in csv_parts(raw) {
        let Some((channel, controller_value)) = entry.split_once(':') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        let Some((controller, value)) = controller_value.split_once('=') else {
            return Err(format!("invalid controller value: {entry}"));
        };
        values.push(ControlValue {
            channel: parse_value(channel.trim(), "inject channel")?,
            controller: parse_value(controller.trim(), "inject controller")?,
            value: parse_value(value.trim(), "inject value")?,
        });
    }
    Ok(values)
}

fn format_control_values(values: &[ControlValue]) -> String {
    values
        .iter()
        .map(|value| format!("{}:{}={}", value.channel, value.controller, value.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_velocity_points(raw: &str) -> Result<Vec<VelocityPoint>, String> {
    let mut points = Vec::new();
    for entry in csv_parts(raw) {
        let Some((input, output)) = entry.split_once(':') else {
            return Err(format!("invalid velocity point: {entry}"));
        };
        points.push(VelocityPoint {
            input: parse_value(input.trim(), "velocity input")?,
            output: parse_value(output.trim(), "velocity output")?,
        });
    }
    Ok(points)
}

fn format_velocity_points(points: &[VelocityPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{}:{}", point.input, point.output))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_key_mappings(raw: &str) -> Result<Vec<KeyMapEntry>, String> {
    let mut mappings = Vec::new();
    for (from, to) in parse_arrow_entries::<u8>(raw, "key mapping")? {
        mappings.push(KeyMapEntry { from, to });
    }
    Ok(mappings)
}

fn format_key_mappings(mappings: &[KeyMapEntry]) -> String {
    mappings
        .iter()
        .map(|mapping| format!("{}->{}", mapping.from, mapping.to))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_key_range(raw: &str) -> Result<Option<KeyRange>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Some((min, max)) = trimmed.split_once('-') else {
        return Err(format!("invalid fold range: {trimmed}"));
    };
    Ok(Some(KeyRange {
        min: parse_value(min.trim(), "fold range min")?,
        max: parse_value(max.trim(), "fold range max")?,
    }))
}

fn format_key_range(range: &Option<KeyRange>) -> String {
    range
        .as_ref()
        .map(|range| format!("{}-{}", range.min, range.max))
        .unwrap_or_default()
}

fn parse_text_kinds(raw: &str) -> Result<Vec<TextKind>, String> {
    csv_parts(raw)
        .map(|entry| parse_serde_enum(entry, "text kind"))
        .collect()
}

fn format_text_kinds(kinds: &[TextKind]) -> String {
    kinds
        .iter()
        .map(format_serde_enum)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_sysex_messages(raw: &str) -> Result<Vec<Vec<u8>>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut messages = Vec::new();
    for message in trimmed
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let mut bytes = Vec::new();
        for token in message
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            bytes.push(parse_u8_token(token, "sysex byte")?);
        }
        messages.push(bytes);
    }
    Ok(messages)
}

fn format_sysex_messages(messages: &[Vec<u8>]) -> String {
    messages
        .iter()
        .map(|message| {
            message
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn ensure_trim_mut(config: &mut MidiFileProcessingConfig) -> &mut TrimProcessingConfig {
    config
        .time
        .trim
        .get_or_insert_with(TrimProcessingConfig::default)
}

fn ensure_tool_for_pass<'a>(
    config: &'a mut MidiFileProcessingConfig,
    pass_key: &str,
) -> &'a mut MidiModifierTool {
    let should_replace = !matches!(config.tools.as_slice(), [tool] if tool_key(tool) == pass_key);
    if should_replace {
        config.tools = vec![tool_for_pass(pass_key)];
    }
    &mut config.tools[0]
}

fn reset_modify_pass_control_fields(app: &App) {
    app.set_modify_range_track_min_text("".into());
    app.set_modify_range_track_max_text("".into());
    app.set_modify_range_channel_min_text("".into());
    app.set_modify_range_channel_max_text("".into());
    app.set_modify_range_key_min_text("".into());
    app.set_modify_range_key_max_text("".into());
    app.set_modify_range_velocity_min_text("".into());
    app.set_modify_range_velocity_max_text("".into());
    app.set_modify_range_tick_start_text("".into());
    app.set_modify_range_tick_end_text("".into());
    app.set_modify_range_event_kinds_text("".into());
    app.set_modify_tempo_mode_text("scale_bpm".into());
    app.set_modify_tempo_flatten_tempo_text("500000".into());
    app.set_modify_tempo_scale_factor_text("1.0".into());
    app.set_modify_tempo_replace_points_text("".into());
    app.set_modify_time_warp_points_text("".into());
    app.set_modify_channel_remap_mappings_text("".into());
    app.set_modify_track_route_mode_text("preserve".into());
    app.set_modify_track_route_mappings_text("".into());
    app.set_modify_program_force_program_text("".into());
    app.set_modify_program_strip_changes_text("off".into());
    app.set_modify_program_startup_programs_text("".into());
    app.set_modify_program_keep_only_startup_text("off".into());
    app.set_modify_control_strip_controllers_text("".into());
    app.set_modify_control_remap_controllers_text("".into());
    app.set_modify_control_scale_controllers_text("".into());
    app.set_modify_control_inject_start_text("".into());
    app.set_modify_pitch_strip_text("off".into());
    app.set_modify_pitch_scale_text("1.0".into());
    app.set_modify_pitch_offset_text("0".into());
    app.set_modify_pitch_min_bend_text("-8192".into());
    app.set_modify_pitch_max_bend_text("8191".into());
    app.set_modify_pitch_reset_at_start_text("off".into());
    app.set_modify_velocity_mode_text("scale".into());
    app.set_modify_velocity_scale_text("1.0".into());
    app.set_modify_velocity_gamma_text("1.0".into());
    app.set_modify_velocity_points_text("".into());
    app.set_modify_note_length_min_ticks_text("".into());
    app.set_modify_note_length_max_ticks_text("".into());
    app.set_modify_note_length_scale_text("1.0".into());
    app.set_modify_note_length_fixed_ticks_text("".into());
    app.set_modify_overlap_repeated_policy_text("close_previous".into());
    app.set_modify_overlap_orphan_policy_text("drop".into());
    app.set_modify_quantize_grid_ticks_text("120".into());
    app.set_modify_quantize_strength_text("1.0".into());
    app.set_modify_quantize_note_ends_text("off".into());
    app.set_modify_quantize_swing_text("0.0".into());
    app.set_modify_humanize_start_jitter_text("8".into());
    app.set_modify_humanize_length_jitter_text("6".into());
    app.set_modify_humanize_velocity_jitter_text("5".into());
    app.set_modify_humanize_seed_text("1".into());
    app.set_modify_key_map_mappings_text("".into());
    app.set_modify_key_map_fold_range_text("".into());
    app.set_modify_key_map_drop_unmapped_text("off".into());
    app.set_modify_dedupe_notes_text("on".into());
    app.set_modify_dedupe_controls_text("off".into());
    app.set_modify_dedupe_tempo_text("off".into());
    app.set_modify_dedupe_meta_text("off".into());
    app.set_modify_meta_strip_all_text("off".into());
    app.set_modify_meta_keep_kinds_text("".into());
    app.set_modify_meta_prefix_track_names_text("".into());
    app.set_modify_sysex_strip_all_text("off".into());
    app.set_modify_sysex_prepend_text("".into());
    app.set_modify_merge_balance_deconflict_text("on".into());
    app.set_modify_merge_balance_strip_duplicate_text("on".into());
    app.set_modify_merge_balance_prefer_first_text("on".into());
    app.set_modify_analysis_guard_min_note_count_text("1".into());
    app.set_modify_analysis_guard_max_note_count_text("".into());
    app.set_modify_analysis_guard_min_track_count_text("".into());
    app.set_modify_analysis_guard_max_track_count_text("".into());
}

fn sync_modify_common_controls(app: &App, config: &MidiFileProcessingConfig) {
    app.set_modify_time_offset_ticks_text(config.time.offset_ticks.to_string().into());
    app.set_modify_time_ppq_override_text(option_text(config.time.ppq_override).into());
    app.set_modify_time_tempo_override_text(option_text(config.time.tempo_override).into());

    if let Some(trim) = &config.time.trim {
        app.set_modify_trim_start_tick_text(trim.start_tick.to_string().into());
        app.set_modify_trim_end_tick_text(option_text(trim.end_tick).into());
        app.set_modify_trim_inject_edge_text(toggle_text(trim.inject_edge_state).into());
        app.set_modify_trim_close_open_text(toggle_text(trim.close_open_notes_at_end).into());
    } else {
        app.set_modify_trim_start_tick_text("".into());
        app.set_modify_trim_end_tick_text("".into());
        app.set_modify_trim_inject_edge_text("on".into());
        app.set_modify_trim_close_open_text("on".into());
    }

    app.set_modify_min_key_text(config.notes.min_key.to_string().into());
    app.set_modify_max_key_text(config.notes.max_key.to_string().into());
    app.set_modify_transpose_text(config.notes.transpose.to_string().into());
    app.set_modify_note_velocity_scale_text(config.notes.velocity_scale.to_string().into());
    app.set_modify_piano_only_text(toggle_text(config.piano_only).into());
    app.set_modify_zero_velocity_mode_text(format_serde_enum(&config.zero_velocity_note_on).into());
    app.set_modify_merge_mode_text(format_serde_enum(&config.merge.mode).into());
    app.set_modify_split_channels_text(toggle_text(config.structure.split_channels).into());
    app.set_modify_collapse_tracks_text(toggle_text(config.structure.collapse_tracks).into());
    app.set_modify_remove_empty_tracks_text(
        toggle_text(config.structure.remove_empty_tracks).into(),
    );
    app.set_modify_drop_orphan_note_offs_text(
        toggle_text(config.structure.drop_orphan_note_offs).into(),
    );
}

fn sync_modify_pass_controls(app: &App, config: &MidiFileProcessingConfig) {
    reset_modify_pass_control_fields(app);

    let [tool] = config.tools.as_slice() else {
        return;
    };

    match tool {
        MidiModifierTool::RangeSelect(tool) => {
            app.set_modify_range_track_min_text(option_text(tool.track_min).into());
            app.set_modify_range_track_max_text(option_text(tool.track_max).into());
            app.set_modify_range_channel_min_text(option_text(tool.channel_min).into());
            app.set_modify_range_channel_max_text(option_text(tool.channel_max).into());
            app.set_modify_range_key_min_text(option_text(tool.key_min).into());
            app.set_modify_range_key_max_text(option_text(tool.key_max).into());
            app.set_modify_range_velocity_min_text(option_text(tool.velocity_min).into());
            app.set_modify_range_velocity_max_text(option_text(tool.velocity_max).into());
            app.set_modify_range_tick_start_text(option_text(tool.tick_start).into());
            app.set_modify_range_tick_end_text(option_text(tool.tick_end).into());
            app.set_modify_range_event_kinds_text(format_event_kinds(&tool.event_kinds).into());
        }
        MidiModifierTool::TempoMap(tool) => match tool {
            TempoMapTool::Flatten { tempo } => {
                app.set_modify_tempo_mode_text("flatten".into());
                app.set_modify_tempo_flatten_tempo_text(tempo.to_string().into());
            }
            TempoMapTool::ScaleBpm { factor } => {
                app.set_modify_tempo_mode_text("scale_bpm".into());
                app.set_modify_tempo_scale_factor_text(factor.to_string().into());
            }
            TempoMapTool::Replace { points } => {
                app.set_modify_tempo_mode_text("replace".into());
                app.set_modify_tempo_replace_points_text(format_tempo_points(points).into());
            }
        },
        MidiModifierTool::TimeWarp(tool) => {
            app.set_modify_time_warp_points_text(format_time_warp_points(&tool.points).into());
        }
        MidiModifierTool::ChannelRemap(tool) => {
            app.set_modify_channel_remap_mappings_text(
                format_channel_mappings(&tool.mappings).into(),
            );
        }
        MidiModifierTool::TrackRoute(tool) => match tool {
            TrackRouteTool::Preserve => {
                app.set_modify_track_route_mode_text("preserve".into());
            }
            TrackRouteTool::CollapseAll => {
                app.set_modify_track_route_mode_text("collapse_all".into());
            }
            TrackRouteTool::SplitByChannel => {
                app.set_modify_track_route_mode_text("split_by_channel".into());
            }
            TrackRouteTool::Map { mappings } => {
                app.set_modify_track_route_mode_text("map".into());
                app.set_modify_track_route_mappings_text(format_track_mappings(mappings).into());
            }
        },
        MidiModifierTool::Program(tool) => {
            app.set_modify_program_force_program_text(option_text(tool.force_program).into());
            app.set_modify_program_strip_changes_text(
                toggle_text(tool.strip_program_changes).into(),
            );
            app.set_modify_program_startup_programs_text(
                format_channel_programs(&tool.startup_programs).into(),
            );
            app.set_modify_program_keep_only_startup_text(
                toggle_text(tool.keep_only_startup).into(),
            );
        }
        MidiModifierTool::ControlChange(tool) => {
            app.set_modify_control_strip_controllers_text(
                format_u8_list(&tool.strip_controllers).into(),
            );
            app.set_modify_control_remap_controllers_text(
                format_controller_mappings(&tool.remap_controllers).into(),
            );
            app.set_modify_control_scale_controllers_text(
                format_controller_scales(&tool.scale_controllers).into(),
            );
            app.set_modify_control_inject_start_text(
                format_control_values(&tool.inject_start).into(),
            );
        }
        MidiModifierTool::PitchBend(tool) => {
            app.set_modify_pitch_strip_text(toggle_text(tool.strip).into());
            app.set_modify_pitch_scale_text(tool.scale.to_string().into());
            app.set_modify_pitch_offset_text(tool.offset.to_string().into());
            app.set_modify_pitch_min_bend_text(tool.min_bend.to_string().into());
            app.set_modify_pitch_max_bend_text(tool.max_bend.to_string().into());
            app.set_modify_pitch_reset_at_start_text(toggle_text(tool.reset_at_start).into());
        }
        MidiModifierTool::VelocityMap(tool) => match tool {
            VelocityMapTool::Scale { scale } => {
                app.set_modify_velocity_mode_text("scale".into());
                app.set_modify_velocity_scale_text(scale.to_string().into());
            }
            VelocityMapTool::Gamma { gamma } => {
                app.set_modify_velocity_mode_text("gamma".into());
                app.set_modify_velocity_gamma_text(gamma.to_string().into());
            }
            VelocityMapTool::Polyline { points } => {
                app.set_modify_velocity_mode_text("polyline".into());
                app.set_modify_velocity_points_text(format_velocity_points(points).into());
            }
        },
        MidiModifierTool::NoteLength(tool) => {
            app.set_modify_note_length_min_ticks_text(option_text(tool.min_ticks).into());
            app.set_modify_note_length_max_ticks_text(option_text(tool.max_ticks).into());
            app.set_modify_note_length_scale_text(option_text(tool.scale).into());
            app.set_modify_note_length_fixed_ticks_text(option_text(tool.fixed_ticks).into());
        }
        MidiModifierTool::OverlapRepair(tool) => {
            app.set_modify_overlap_repeated_policy_text(
                format_serde_enum(&tool.repeated_note_on).into(),
            );
            app.set_modify_overlap_orphan_policy_text(
                format_serde_enum(&tool.orphan_note_offs).into(),
            );
        }
        MidiModifierTool::Quantize(tool) => {
            app.set_modify_quantize_grid_ticks_text(tool.grid_ticks.to_string().into());
            app.set_modify_quantize_strength_text(tool.strength.to_string().into());
            app.set_modify_quantize_note_ends_text(toggle_text(tool.quantize_note_ends).into());
            app.set_modify_quantize_swing_text(tool.swing.to_string().into());
        }
        MidiModifierTool::Humanize(tool) => {
            app.set_modify_humanize_start_jitter_text(tool.start_jitter.to_string().into());
            app.set_modify_humanize_length_jitter_text(tool.length_jitter.to_string().into());
            app.set_modify_humanize_velocity_jitter_text(tool.velocity_jitter.to_string().into());
            app.set_modify_humanize_seed_text(tool.seed.to_string().into());
        }
        MidiModifierTool::KeyMap(tool) => {
            app.set_modify_key_map_mappings_text(format_key_mappings(&tool.mappings).into());
            app.set_modify_key_map_fold_range_text(format_key_range(&tool.fold_to_range).into());
            app.set_modify_key_map_drop_unmapped_text(toggle_text(tool.drop_unmapped).into());
        }
        MidiModifierTool::Dedupe(tool) => {
            app.set_modify_dedupe_notes_text(toggle_text(tool.notes).into());
            app.set_modify_dedupe_controls_text(toggle_text(tool.controls).into());
            app.set_modify_dedupe_tempo_text(toggle_text(tool.tempo).into());
            app.set_modify_dedupe_meta_text(toggle_text(tool.meta).into());
        }
        MidiModifierTool::MetaText(tool) => {
            app.set_modify_meta_strip_all_text(toggle_text(tool.strip_all_text).into());
            app.set_modify_meta_keep_kinds_text(format_text_kinds(&tool.keep_kinds).into());
            app.set_modify_meta_prefix_track_names_text(
                tool.prefix_track_names.clone().unwrap_or_default().into(),
            );
        }
        MidiModifierTool::Sysex(tool) => {
            app.set_modify_sysex_strip_all_text(toggle_text(tool.strip_all).into());
            app.set_modify_sysex_prepend_text(format_sysex_messages(&tool.prepend).into());
        }
        MidiModifierTool::MergeBalance(tool) => {
            app.set_modify_merge_balance_deconflict_text(
                toggle_text(tool.deconflict_channels).into(),
            );
            app.set_modify_merge_balance_strip_duplicate_text(
                toggle_text(tool.strip_duplicate_start_state).into(),
            );
            app.set_modify_merge_balance_prefer_first_text(
                toggle_text(tool.prefer_first_tempo_map).into(),
            );
        }
        MidiModifierTool::SharedMetadataTrack(_) => {}
        MidiModifierTool::AnalysisGuard(tool) => {
            app.set_modify_analysis_guard_min_note_count_text(
                option_text(tool.min_note_count).into(),
            );
            app.set_modify_analysis_guard_max_note_count_text(
                option_text(tool.max_note_count).into(),
            );
            app.set_modify_analysis_guard_min_track_count_text(
                option_text(tool.min_track_count).into(),
            );
            app.set_modify_analysis_guard_max_track_count_text(
                option_text(tool.max_track_count).into(),
            );
        }
    }
}

fn sync_modify_controls(app: &App, config: &MidiFileProcessingConfig) {
    sync_modify_common_controls(app, config);
    sync_modify_pass_controls(app, config);
}

fn update_modify_control(app: &App, key: &str, value: &str) -> Result<(), String> {
    let mut config = fallback_modify_config(app);

    match key {
        "merge.mode" => {
            config.merge.mode = parse_serde_enum(value, "merge mode")?;
        }
        "time.offset_ticks" => {
            config.time.offset_ticks = parse_value(value, "time offset ticks")?;
        }
        "time.ppq_override" => {
            config.time.ppq_override = parse_optional_value(value, "PPQ override")?;
        }
        "time.tempo_override" => {
            config.time.tempo_override = parse_optional_value(value, "tempo override")?;
        }
        "time.trim.start_tick" => {
            if value.trim().is_empty() {
                config.time.trim = None;
            } else {
                ensure_trim_mut(&mut config).start_tick = parse_value(value, "trim start tick")?;
            }
        }
        "time.trim.end_tick" => {
            ensure_trim_mut(&mut config).end_tick = parse_optional_value(value, "trim end tick")?;
        }
        "time.trim.inject_edge_state" => {
            ensure_trim_mut(&mut config).inject_edge_state =
                parse_bool_toggle(value, "trim edge state")?;
        }
        "time.trim.close_open_notes_at_end" => {
            ensure_trim_mut(&mut config).close_open_notes_at_end =
                parse_bool_toggle(value, "trim note closing")?;
        }
        "piano_only" => {
            config.piano_only = parse_bool_toggle(value, "piano-only toggle")?;
        }
        "zero_velocity_note_on" => {
            config.zero_velocity_note_on = parse_serde_enum(value, "zero-velocity mode")?;
        }
        "notes.min_key" => {
            config.notes.min_key = parse_value(value, "minimum key")?;
        }
        "notes.max_key" => {
            config.notes.max_key = parse_value(value, "maximum key")?;
        }
        "notes.transpose" => {
            config.notes.transpose = parse_value(value, "transpose")?;
        }
        "notes.velocity_scale" => {
            config.notes.velocity_scale = parse_value(value, "velocity scale")?;
        }
        "structure.split_channels" => {
            config.structure.split_channels = parse_bool_toggle(value, "split channels toggle")?;
        }
        "structure.collapse_tracks" => {
            config.structure.collapse_tracks = parse_bool_toggle(value, "collapse tracks toggle")?;
        }
        "structure.remove_empty_tracks" => {
            config.structure.remove_empty_tracks =
                parse_bool_toggle(value, "remove empty tracks toggle")?;
        }
        "structure.drop_orphan_note_offs" => {
            config.structure.drop_orphan_note_offs =
                parse_bool_toggle(value, "drop orphan note-offs toggle")?;
        }
        "range.track_min" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.track_min = parse_optional_value(value, "range track min")?;
            }
        }
        "range.track_max" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.track_max = parse_optional_value(value, "range track max")?;
            }
        }
        "range.channel_min" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.channel_min = parse_optional_value(value, "range channel min")?;
            }
        }
        "range.channel_max" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.channel_max = parse_optional_value(value, "range channel max")?;
            }
        }
        "range.key_min" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.key_min = parse_optional_value(value, "range key min")?;
            }
        }
        "range.key_max" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.key_max = parse_optional_value(value, "range key max")?;
            }
        }
        "range.velocity_min" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.velocity_min = parse_optional_value(value, "range velocity min")?;
            }
        }
        "range.velocity_max" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.velocity_max = parse_optional_value(value, "range velocity max")?;
            }
        }
        "range.tick_start" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.tick_start = parse_optional_value(value, "range tick start")?;
            }
        }
        "range.tick_end" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.tick_end = parse_optional_value(value, "range tick end")?;
            }
        }
        "range.event_kinds" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.event_kinds = parse_event_kinds(value)?;
            }
        }
        "tempo.mode" => {
            let tool = match value.trim() {
                "flatten" => MidiModifierTool::TempoMap(TempoMapTool::Flatten { tempo: 500000 }),
                "scale_bpm" => MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor: 1.0 }),
                "replace" => {
                    MidiModifierTool::TempoMap(TempoMapTool::Replace { points: Vec::new() })
                }
                other => return Err(format!("invalid tempo mode: {other}")),
            };
            config.tools = vec![tool];
        }
        "tempo.flatten_tempo" => {
            config.tools = vec![MidiModifierTool::TempoMap(TempoMapTool::Flatten {
                tempo: parse_value(value, "flatten tempo")?,
            })];
        }
        "tempo.scale_factor" => {
            config.tools = vec![MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm {
                factor: parse_value(value, "tempo scale factor")?,
            })];
        }
        "tempo.replace_points" => {
            config.tools = vec![MidiModifierTool::TempoMap(TempoMapTool::Replace {
                points: parse_tempo_points(value)?,
            })];
        }
        "time_warp.points" => {
            if let MidiModifierTool::TimeWarp(tool) = ensure_tool_for_pass(&mut config, "time_warp")
            {
                tool.points = parse_time_warp_points(value)?;
            }
        }
        "channel_remap.mappings" => {
            if let MidiModifierTool::ChannelRemap(tool) =
                ensure_tool_for_pass(&mut config, "channel_remap")
            {
                tool.mappings = parse_channel_mappings(value)?;
            }
        }
        "track_route.mode" => {
            let tool = match value.trim() {
                "preserve" => MidiModifierTool::TrackRoute(TrackRouteTool::Preserve),
                "collapse_all" => MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll),
                "split_by_channel" => MidiModifierTool::TrackRoute(TrackRouteTool::SplitByChannel),
                "map" => MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                    mappings: Vec::new(),
                }),
                other => return Err(format!("invalid track-route mode: {other}")),
            };
            config.tools = vec![tool];
        }
        "track_route.mappings" => {
            config.tools = vec![MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                mappings: parse_track_mappings(value)?,
            })];
        }
        "program.force_program" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.force_program = parse_optional_value(value, "force program")?;
            }
        }
        "program.strip_program_changes" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.strip_program_changes = parse_bool_toggle(value, "strip program changes")?;
            }
        }
        "program.startup_programs" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.startup_programs = parse_channel_programs(value)?;
            }
        }
        "program.keep_only_startup" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.keep_only_startup = parse_bool_toggle(value, "keep only startup")?;
            }
        }
        "control_change.strip_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.strip_controllers = parse_u8_list(value, "controller number")?;
            }
        }
        "control_change.remap_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.remap_controllers = parse_controller_mappings(value)?;
            }
        }
        "control_change.scale_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.scale_controllers = parse_controller_scales(value)?;
            }
        }
        "control_change.inject_start" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.inject_start = parse_control_values(value)?;
            }
        }
        "pitch_bend.strip" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.strip = parse_bool_toggle(value, "pitch-bend strip toggle")?;
            }
        }
        "pitch_bend.scale" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.scale = parse_value(value, "pitch-bend scale")?;
            }
        }
        "pitch_bend.offset" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.offset = parse_value(value, "pitch-bend offset")?;
            }
        }
        "pitch_bend.min_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.min_bend = parse_value(value, "minimum bend")?;
            }
        }
        "pitch_bend.max_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.max_bend = parse_value(value, "maximum bend")?;
            }
        }
        "pitch_bend.reset_at_start" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.reset_at_start = parse_bool_toggle(value, "reset-at-start toggle")?;
            }
        }
        "velocity_map.mode" => {
            let tool = match value.trim() {
                "scale" => MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 }),
                "gamma" => MidiModifierTool::VelocityMap(VelocityMapTool::Gamma { gamma: 1.0 }),
                "polyline" => {
                    MidiModifierTool::VelocityMap(VelocityMapTool::Polyline { points: Vec::new() })
                }
                other => return Err(format!("invalid velocity-map mode: {other}")),
            };
            config.tools = vec![tool];
        }
        "velocity_map.scale" => {
            config.tools = vec![MidiModifierTool::VelocityMap(VelocityMapTool::Scale {
                scale: parse_value(value, "velocity scale")?,
            })];
        }
        "velocity_map.gamma" => {
            config.tools = vec![MidiModifierTool::VelocityMap(VelocityMapTool::Gamma {
                gamma: parse_value(value, "velocity gamma")?,
            })];
        }
        "velocity_map.points" => {
            config.tools = vec![MidiModifierTool::VelocityMap(VelocityMapTool::Polyline {
                points: parse_velocity_points(value)?,
            })];
        }
        "note_length.min_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.min_ticks = parse_optional_value(value, "minimum note length")?;
            }
        }
        "note_length.max_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.max_ticks = parse_optional_value(value, "maximum note length")?;
            }
        }
        "note_length.scale" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.scale = parse_optional_value(value, "note-length scale")?;
            }
        }
        "note_length.fixed_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.fixed_ticks = parse_optional_value(value, "fixed note length")?;
            }
        }
        "overlap.repeated_note_on" => {
            if let MidiModifierTool::OverlapRepair(tool) =
                ensure_tool_for_pass(&mut config, "overlap_repair")
            {
                tool.repeated_note_on = parse_serde_enum(value, "repeated-note policy")?;
            }
        }
        "overlap.orphan_note_offs" => {
            if let MidiModifierTool::OverlapRepair(tool) =
                ensure_tool_for_pass(&mut config, "overlap_repair")
            {
                tool.orphan_note_offs = parse_serde_enum(value, "orphan-note-off policy")?;
            }
        }
        "quantize.grid_ticks" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.grid_ticks = parse_value(value, "quantize grid")?;
            }
        }
        "quantize.strength" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.strength = parse_value(value, "quantize strength")?;
            }
        }
        "quantize.note_ends" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.quantize_note_ends = parse_bool_toggle(value, "quantize note-ends toggle")?;
            }
        }
        "quantize.swing" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.swing = parse_value(value, "quantize swing")?;
            }
        }
        "humanize.start_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.start_jitter = parse_value(value, "humanize start jitter")?;
            }
        }
        "humanize.length_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.length_jitter = parse_value(value, "humanize length jitter")?;
            }
        }
        "humanize.velocity_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.velocity_jitter = parse_value(value, "humanize velocity jitter")?;
            }
        }
        "humanize.seed" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.seed = parse_value(value, "humanize seed")?;
            }
        }
        "key_map.mappings" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.mappings = parse_key_mappings(value)?;
            }
        }
        "key_map.fold_range" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.fold_to_range = parse_key_range(value)?;
            }
        }
        "key_map.drop_unmapped" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.drop_unmapped = parse_bool_toggle(value, "drop-unmapped toggle")?;
            }
        }
        "dedupe.notes" => {
            if let MidiModifierTool::Dedupe(tool) = ensure_tool_for_pass(&mut config, "dedupe") {
                tool.notes = parse_bool_toggle(value, "dedupe notes toggle")?;
            }
        }
        "dedupe.controls" => {
            if let MidiModifierTool::Dedupe(tool) = ensure_tool_for_pass(&mut config, "dedupe") {
                tool.controls = parse_bool_toggle(value, "dedupe controls toggle")?;
            }
        }
        "dedupe.tempo" => {
            if let MidiModifierTool::Dedupe(tool) = ensure_tool_for_pass(&mut config, "dedupe") {
                tool.tempo = parse_bool_toggle(value, "dedupe tempo toggle")?;
            }
        }
        "dedupe.meta" => {
            if let MidiModifierTool::Dedupe(tool) = ensure_tool_for_pass(&mut config, "dedupe") {
                tool.meta = parse_bool_toggle(value, "dedupe meta toggle")?;
            }
        }
        "meta.strip_all" => {
            if let MidiModifierTool::MetaText(tool) = ensure_tool_for_pass(&mut config, "meta_text")
            {
                tool.strip_all_text = parse_bool_toggle(value, "meta strip-all toggle")?;
            }
        }
        "meta.keep_kinds" => {
            if let MidiModifierTool::MetaText(tool) = ensure_tool_for_pass(&mut config, "meta_text")
            {
                tool.keep_kinds = parse_text_kinds(value)?;
            }
        }
        "meta.prefix_track_names" => {
            if let MidiModifierTool::MetaText(tool) = ensure_tool_for_pass(&mut config, "meta_text")
            {
                tool.prefix_track_names =
                    Some(value.trim().to_string()).filter(|text| !text.is_empty());
            }
        }
        "sysex.strip_all" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(&mut config, "sysex") {
                tool.strip_all = parse_bool_toggle(value, "sysex strip-all toggle")?;
            }
        }
        "sysex.prepend" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(&mut config, "sysex") {
                tool.prepend = parse_sysex_messages(value)?;
            }
        }
        "merge_balance.deconflict_channels" => {
            if let MidiModifierTool::MergeBalance(tool) =
                ensure_tool_for_pass(&mut config, "merge_balance")
            {
                tool.deconflict_channels = parse_bool_toggle(value, "deconflict toggle")?;
            }
        }
        "merge_balance.strip_duplicate_start_state" => {
            if let MidiModifierTool::MergeBalance(tool) =
                ensure_tool_for_pass(&mut config, "merge_balance")
            {
                tool.strip_duplicate_start_state =
                    parse_bool_toggle(value, "strip-duplicate-state toggle")?;
            }
        }
        "merge_balance.prefer_first_tempo_map" => {
            if let MidiModifierTool::MergeBalance(tool) =
                ensure_tool_for_pass(&mut config, "merge_balance")
            {
                tool.prefer_first_tempo_map =
                    parse_bool_toggle(value, "prefer-first-tempo toggle")?;
            }
        }
        "analysis_guard.min_note_count" => {
            if let MidiModifierTool::AnalysisGuard(tool) =
                ensure_tool_for_pass(&mut config, "analysis_guard")
            {
                tool.min_note_count = parse_optional_value(value, "minimum note count")?;
            }
        }
        "analysis_guard.max_note_count" => {
            if let MidiModifierTool::AnalysisGuard(tool) =
                ensure_tool_for_pass(&mut config, "analysis_guard")
            {
                tool.max_note_count = parse_optional_value(value, "maximum note count")?;
            }
        }
        "analysis_guard.min_track_count" => {
            if let MidiModifierTool::AnalysisGuard(tool) =
                ensure_tool_for_pass(&mut config, "analysis_guard")
            {
                tool.min_track_count = parse_optional_value(value, "minimum track count")?;
            }
        }
        "analysis_guard.max_track_count" => {
            if let MidiModifierTool::AnalysisGuard(tool) =
                ensure_tool_for_pass(&mut config, "analysis_guard")
            {
                tool.max_track_count = parse_optional_value(value, "maximum track count")?;
            }
        }
        other => return Err(format!("unknown modify control: {other}")),
    }

    set_modify_config(app, &config);
    Ok(())
}

fn pass_metadata(pass_key: &str) -> (&'static str, &'static str, &'static str) {
    match pass_key {
        "range_select" => (
            "Range Select",
            "Limit later tools to a subset of tracks, channels, keys, ticks, velocities, or event kinds.",
            "Useful as the first tool in a multi-pass pipeline when you only want to touch one region of the file.",
        ),
        "tempo_map" => (
            "Tempo Map",
            "Flatten, scale, or fully replace tempo events while leaving the rest of the file intact.",
            "The preset starts with a no-op tempo scale so you can switch modes or edit values in place.",
        ),
        "time_warp" => (
            "Time Warp",
            "Remap absolute ticks through custom control points for rubato fixes or structural timing edits.",
            "Edit the `points` array with `source_tick` and `dest_tick` values in ascending order.",
        ),
        "channel_remap" => (
            "Channel Remap",
            "Move note and controller data between MIDI channels without rebuilding the file by hand.",
            "Add mapping entries like `{ \"from\": 0, \"to\": 1 }` to retarget channels.",
        ),
        "track_route" => (
            "Track Route",
            "Preserve, collapse, split, or explicitly remap tracks before the file is written back out.",
            "The default preset preserves tracks. Switch the `mode` field to `collapse_all`, `split_by_channel`, or `map`.",
        ),
        "program" => (
            "Program",
            "Force startup programs, strip later program changes, or keep only the initial setup state.",
            "Use this when a synth or export target needs deterministic instrument assignments.",
        ),
        "control_change" => (
            "Control Change",
            "Strip, remap, scale, or inject controller values at the start of playback.",
            "Good for fixing sustain pedals, volume curves, expression, or target-device controller layouts.",
        ),
        "pitch_bend" => (
            "Pitch Bend",
            "Scale, offset, clamp, reset, or strip pitch-bend data without touching note timing.",
            "The preset keeps bends intact with full-range limits so you can dial in corrections safely.",
        ),
        "velocity_map" => (
            "Velocity Map",
            "Re-shape note-on velocities with a scale, gamma curve, or custom polyline.",
            "Start with a simple scale, or replace the tool variant in JSON for more detailed curves.",
        ),
        "note_length" => (
            "Note Length",
            "Clamp, scale, or replace note durations across the selected note set.",
            "Combine with `range_select` if only part of the arrangement should be resized.",
        ),
        "overlap_repair" => (
            "Overlap Repair",
            "Close repeated note-ons cleanly and drop orphan note-offs that can confuse playback engines.",
            "This is a good cleanup pass before export or before applying timing-sensitive tools.",
        ),
        "quantize" => (
            "Quantize",
            "Snap notes toward a grid with optional swing and note-end handling.",
            "The default preset uses a 120-tick grid at full strength; edit it for your project PPQ and feel.",
        ),
        "humanize" => (
            "Humanize",
            "Add deterministic timing, duration, and velocity variation using a reproducible seed.",
            "Use small values first. The preset is intentionally gentle so you can hear the change without wrecking alignment.",
        ),
        "key_map" => (
            "Key Map",
            "Remap source notes to new pitches, fold them into a target range, or drop unmapped notes.",
            "Useful for keyboard reductions, drum remaps, and narrowing orchestral parts into a playable register.",
        ),
        "dedupe" => (
            "Dedupe",
            "Remove duplicate notes, controllers, tempo changes, or meta events after merges or cleanup.",
            "The preset focuses on note duplicates first; expand the booleans if the file has duplicated control state too.",
        ),
        "meta_text" => (
            "Meta Text",
            "Strip lyric/text metadata or rewrite track-name prefixes without touching musical events.",
            "Helpful when preparing clean delivery files or standardizing merged project metadata.",
        ),
        "sysex" => (
            "SysEx",
            "Strip SysEx entirely or prepend specific setup messages at the start of the file.",
            "Use raw byte arrays in decimal form inside `prepend`, for example `[[67,16,76]]`.",
        ),
        "merge_balance" => (
            "Merge Balance",
            "Clean up collisions created by multi-input merges, including duplicate startup state and channel conflicts.",
            "This matters most when you feed more than one input file into the processing config.",
        ),
        "analysis_guard" => (
            "Analysis Guard",
            "Abort the job if the processed result violates simple note-count or track-count thresholds.",
            "This is useful as a safety rail in automated pipelines where a bad transform should fail loudly.",
        ),
        _ => (
            "Custom Pipeline",
            "The current JSON does not map cleanly to one preset button.",
            "You can still run the job. Use a pass preset again if you want to reseed the draft.",
        ),
    }
}

fn tool_key(tool: &MidiModifierTool) -> &'static str {
    match tool {
        MidiModifierTool::RangeSelect(_) => "range_select",
        MidiModifierTool::TempoMap(_) => "tempo_map",
        MidiModifierTool::TimeWarp(_) => "time_warp",
        MidiModifierTool::ChannelRemap(_) => "channel_remap",
        MidiModifierTool::TrackRoute(_) => "track_route",
        MidiModifierTool::Program(_) => "program",
        MidiModifierTool::ControlChange(_) => "control_change",
        MidiModifierTool::PitchBend(_) => "pitch_bend",
        MidiModifierTool::VelocityMap(_) => "velocity_map",
        MidiModifierTool::NoteLength(_) => "note_length",
        MidiModifierTool::OverlapRepair(_) => "overlap_repair",
        MidiModifierTool::Quantize(_) => "quantize",
        MidiModifierTool::Humanize(_) => "humanize",
        MidiModifierTool::KeyMap(_) => "key_map",
        MidiModifierTool::Dedupe(_) => "dedupe",
        MidiModifierTool::MetaText(_) => "meta_text",
        MidiModifierTool::Sysex(_) => "sysex",
        MidiModifierTool::MergeBalance(_) => "merge_balance",
        MidiModifierTool::SharedMetadataTrack(_) => "shared_metadata_track",
        MidiModifierTool::AnalysisGuard(_) => "analysis_guard",
    }
}

fn tool_for_pass(pass_key: &str) -> MidiModifierTool {
    match pass_key {
        "range_select" => MidiModifierTool::RangeSelect(RangeSelectTool::default()),
        "tempo_map" => MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor: 1.0 }),
        "time_warp" => MidiModifierTool::TimeWarp(TimeWarpTool::default()),
        "channel_remap" => MidiModifierTool::ChannelRemap(ChannelRemapTool::default()),
        "track_route" => MidiModifierTool::TrackRoute(TrackRouteTool::Preserve),
        "program" => MidiModifierTool::Program(ProgramTool {
            force_program: None,
            strip_program_changes: false,
            startup_programs: Vec::new(),
            keep_only_startup: false,
        }),
        "control_change" => MidiModifierTool::ControlChange(ControlChangeTool::default()),
        "pitch_bend" => MidiModifierTool::PitchBend(PitchBendTool {
            strip: false,
            scale: 1.0,
            offset: 0,
            min_bend: -8192,
            max_bend: 8191,
            reset_at_start: false,
        }),
        "velocity_map" => MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 }),
        "note_length" => MidiModifierTool::NoteLength(NoteLengthTool {
            min_ticks: None,
            max_ticks: None,
            scale: Some(1.0),
            fixed_ticks: None,
        }),
        "overlap_repair" => MidiModifierTool::OverlapRepair(OverlapRepairTool::default()),
        "quantize" => MidiModifierTool::Quantize(QuantizeTool {
            grid_ticks: 120,
            strength: 1.0,
            quantize_note_ends: false,
            swing: 0.0,
        }),
        "humanize" => MidiModifierTool::Humanize(HumanizeTool {
            start_jitter: 8,
            length_jitter: 6,
            velocity_jitter: 5,
            seed: 1,
        }),
        "key_map" => MidiModifierTool::KeyMap(KeyMapTool::default()),
        "dedupe" => MidiModifierTool::Dedupe(DedupeTool {
            notes: true,
            controls: false,
            tempo: false,
            meta: false,
        }),
        "meta_text" => MidiModifierTool::MetaText(MetaTextTool::default()),
        "sysex" => MidiModifierTool::Sysex(SysexTool::default()),
        "merge_balance" => MidiModifierTool::MergeBalance(MergeBalanceTool {
            deconflict_channels: true,
            strip_duplicate_start_state: true,
            prefer_first_tempo_map: true,
        }),
        "shared_metadata_track" => MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
            target_track_index: 0,
            move_tempo_events: true,
            move_time_signatures: true,
            move_key_signatures: true,
            move_text_events: false,
        }),
        "analysis_guard" => MidiModifierTool::AnalysisGuard(AnalysisGuardTool {
            min_note_count: Some(1),
            max_note_count: None,
            min_track_count: None,
            max_track_count: None,
        }),
        _ => MidiModifierTool::Quantize(QuantizeTool {
            grid_ticks: 120,
            strength: 1.0,
            quantize_note_ends: false,
            swing: 0.0,
        }),
    }
}

fn default_modify_config(pass_key: &str) -> MidiFileProcessingConfig {
    let mut config = MidiFileProcessingConfig::default();
    config.tools = vec![tool_for_pass(pass_key)];
    config
}

fn load_modify_pass_into_app(app: &App, pass_key: &str) {
    let mut config = fallback_modify_config(app);
    config.tools = vec![tool_for_pass(pass_key)];
    set_modify_config(app, &config);
}

fn sync_modify_pass_metadata(app: &App, pass_key: &str) {
    let (title, description, hint) = pass_metadata(pass_key);
    app.set_modify_pass_key_text(pass_key.into());
    app.set_modify_pass_title_text(title.into());
    app.set_modify_pass_description_text(description.into());
    app.set_modify_pass_hint_text(hint.into());
}

fn sync_modify_pass_metadata_from_config(app: &App, config: &MidiFileProcessingConfig) {
    match config.tools.as_slice() {
        [tool] => sync_modify_pass_metadata(app, tool_key(tool)),
        [] => sync_modify_pass_metadata(app, "custom"),
        _ => sync_modify_pass_metadata(app, "custom"),
    }
}

fn validate_modify_config(app: &App) {
    match parse_modify_config_text(app.get_modify_config_text().as_str()) {
        Ok(config) => {
            app.set_modify_last_valid_config_text(serialize_modify_config(&config).into());
            sync_modify_pass_metadata_from_config(app, &config);
            sync_modify_controls(app, &config);
            app.set_modify_config_valid(true);
            let tool_count = config.tools.len();
            let status = if tool_count == 0 {
                "Valid JSON. No modifier tools configured; only the shared process config will run."
                    .to_string()
            } else {
                format!(
                    "Valid JSON. {} tool{} configured.",
                    tool_count,
                    if tool_count == 1 { "" } else { "s" }
                )
            };
            app.set_modify_config_status_text(status.into());
        }
        Err(error) => {
            app.set_modify_config_valid(false);
            app.set_modify_config_status_text(format!("JSON parse error: {error}").into());
        }
    }
}

fn update_merge_control(app: &App, key: &str, value: &str) {
    match key {
        "layout" => app.set_merge_layout_text(value.into()),
        "metadata_mode" => app.set_merge_metadata_mode_text(value.into()),
        "remove_empty_tracks" => app.set_merge_remove_empty_tracks_text(value.into()),
        "strip_duplicate_start" => app.set_merge_strip_duplicate_start_text(value.into()),
        "prefer_first_tempo" => app.set_merge_prefer_first_tempo_text(value.into()),
        "deconflict_channels" => app.set_merge_deconflict_channels_text(value.into()),
        "dedupe_meta" => app.set_merge_dedupe_meta_text(value.into()),
        "dedupe_controls" => app.set_merge_dedupe_controls_text(value.into()),
        "dedupe_notes" => app.set_merge_dedupe_notes_text(value.into()),
        _ => {}
    }
}

fn build_merge_processing_config(app: &App) -> MidiFileProcessingConfig {
    let mut config = MidiFileProcessingConfig::default();
    config.merge.mode = parse_serde_enum(app.get_merge_layout_text().as_str(), "merge layout mode")
        .unwrap_or_default();
    config.structure.remove_empty_tracks = parse_bool_toggle(
        app.get_merge_remove_empty_tracks_text().as_str(),
        "remove empty tracks toggle",
    )
    .unwrap_or(true);

    let metadata_mode = app.get_merge_metadata_mode_text();
    if metadata_mode.as_str() != "keep" {
        config.tools.push(MidiModifierTool::SharedMetadataTrack(
            SharedMetadataTrackTool {
                target_track_index: 0,
                move_tempo_events: true,
                move_time_signatures: true,
                move_key_signatures: true,
                move_text_events: metadata_mode.as_str() == "all_text_meta",
            },
        ));
    }

    let merge_balance = MergeBalanceTool {
        deconflict_channels: parse_bool_toggle(
            app.get_merge_deconflict_channels_text().as_str(),
            "deconflict channels toggle",
        )
        .unwrap_or(false),
        strip_duplicate_start_state: parse_bool_toggle(
            app.get_merge_strip_duplicate_start_text().as_str(),
            "strip duplicate start toggle",
        )
        .unwrap_or(true),
        prefer_first_tempo_map: parse_bool_toggle(
            app.get_merge_prefer_first_tempo_text().as_str(),
            "prefer first tempo toggle",
        )
        .unwrap_or(true),
    };
    if merge_balance.deconflict_channels
        || merge_balance.strip_duplicate_start_state
        || merge_balance.prefer_first_tempo_map
    {
        config
            .tools
            .push(MidiModifierTool::MergeBalance(merge_balance));
    }

    let dedupe_notes = parse_bool_toggle(
        app.get_merge_dedupe_notes_text().as_str(),
        "dedupe notes toggle",
    )
    .unwrap_or(false);
    let dedupe_controls = parse_bool_toggle(
        app.get_merge_dedupe_controls_text().as_str(),
        "dedupe controls toggle",
    )
    .unwrap_or(false);
    let dedupe_meta = parse_bool_toggle(
        app.get_merge_dedupe_meta_text().as_str(),
        "dedupe meta toggle",
    )
    .unwrap_or(true);
    if dedupe_notes || dedupe_controls || dedupe_meta {
        config.tools.push(MidiModifierTool::Dedupe(DedupeTool {
            notes: dedupe_notes,
            controls: dedupe_controls,
            tempo: dedupe_meta,
            meta: dedupe_meta,
        }));
    }

    config
}

fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn default_merge_output_path(inputs: &[PathBuf]) -> PathBuf {
    let Some(first) = inputs.first() else {
        return PathBuf::from("merged-output.mid");
    };
    let stem = first
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("merged-output");
    let suffix = if inputs.len() > 1 {
        format!("-plus-{}", inputs.len() - 1)
    } else {
        String::new()
    };
    let file_name = format!("{stem}{suffix}-merged.mid");
    first
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

fn normalize_merge_output_path(path: &Path) -> PathBuf {
    let mut normalized = path.to_path_buf();
    let has_midi_extension = normalized
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mid") || ext.eq_ignore_ascii_case("midi"));
    if !has_midi_extension {
        normalized.set_extension("mid");
    }
    normalized
}

fn current_merge_output_path(app: &App) -> Result<PathBuf, String> {
    let raw = app.get_merge_output_path_text();
    if raw.is_empty() {
        return Err("choose an output path".into());
    }
    Ok(normalize_merge_output_path(Path::new(raw.as_str())))
}

fn set_merge_ui_failure(app: &App, message: &str) {
    app.set_merge_job_active(false);
    app.set_merge_progress(0.0);
    app.set_merge_status_text("Failed".into());
    app.set_merge_detail_text(message.into());
}

fn default_modify_output_path(selected_midi_name: &str) -> PathBuf {
    let selected_path = PathBuf::from(selected_midi_name);
    let stem = selected_path
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("modified-output");
    let file_name = format!("{stem}-modified.mid");
    selected_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

fn normalize_modify_output_path(path: &std::path::Path) -> PathBuf {
    let mut normalized = path.to_path_buf();
    let has_midi_extension = normalized
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mid") || ext.eq_ignore_ascii_case("midi"));
    if !has_midi_extension {
        normalized.set_extension("mid");
    }
    normalized
}

fn current_modify_output_path(app: &App) -> Result<PathBuf, String> {
    let raw = app.get_modify_output_path_text();
    if raw.is_empty() {
        return Err("choose an output path".into());
    }
    Ok(normalize_modify_output_path(std::path::Path::new(
        raw.as_str(),
    )))
}

fn parse_modify_config(app: &App) -> Result<MidiFileProcessingConfig, String> {
    parse_modify_config_text(app.get_modify_config_text().as_str())
}

fn set_modify_ui_failure(app: &App, message: &str) {
    app.set_modify_job_active(false);
    app.set_modify_progress(0.0);
    app.set_modify_status_text("Failed".into());
    app.set_modify_detail_text(message.into());
}

fn events_error_message(events: &[CoreEvent]) -> Option<String> {
    events.iter().find_map(|event| match event {
        CoreEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    })
}

fn install_midi_process_listener(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    std::thread::spawn(move || {
        for event in receiver {
            let is_modify_event = matches!(
                event,
                CoreEvent::MidiProcess { .. } | CoreEvent::MidiProcessStatus { .. }
            );
            if !is_modify_event {
                continue;
            }

            shared_state
                .lock()
                .expect("shared UI state mutex poisoned")
                .reduce_events(std::slice::from_ref(&event));

            let event_for_ui = event.clone();
            let shared_state = Arc::clone(&shared_state);
            let _ = app_weak.upgrade_in_event_loop(move |app| {
                apply_events_to_app(&app, &shared_state, std::slice::from_ref(&event_for_ui));
                app.window().request_redraw();
            });
        }
    });
}

fn envelope_curve_from_label(label: &str) -> EnvelopeCurveType {
    match label {
        "linear" => EnvelopeCurveType::Linear,
        _ => EnvelopeCurveType::Exponential,
    }
}

fn bool_from_label(label: &str) -> bool {
    matches!(label, "on" | "yes" | "true" | "1")
}

fn normalize_top_bar_color(value: &str) -> Option<String> {
    PfaKeyboardProjectorConfig::normalize_top_bar_color(value)
}

fn palette_mut(scene: &mut SceneConfig) -> Option<&mut NotePaletteConfig> {
    match scene {
        SceneConfig::TwoD(config) => match &mut config.notes {
            NoteProjectorConfig::Flat(notes) => Some(&mut notes.palette),
            NoteProjectorConfig::Pfa(notes) => Some(&mut notes.palette),
        },
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            Some(&mut config.palette)
        }
    }
}

fn ptc_mut(scene: &mut SceneConfig) -> Option<&mut PianoTrailClassicSceneConfig> {
    match scene {
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => Some(config),
        _ => None,
    }
}

fn update_pfa_note_bool(
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

fn update_pfa_keyboard_bool(
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

fn update_ptc_bool(
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

fn update_ptc_f32(
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

fn clear_export_state(export_state: &Arc<Mutex<RenderExportCoordinator>>) {
    *export_state
        .lock()
        .expect("render export coordinator mutex poisoned") = RenderExportCoordinator::default();
}

fn fail_export(app: &App, export_state: &Arc<Mutex<RenderExportCoordinator>>, message: String) {
    clear_export_state(export_state);
    set_export_status(app, "Failed", message, 0.0);
    app.window().request_redraw();
}

fn start_render_export_jobs(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) -> Result<(), String> {
    let snapshot = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .clone()
        .ok_or_else(|| "load a MIDI before exporting".to_string())?;
    if snapshot.midi_path.is_none() {
        return Err("load a MIDI before exporting".into());
    }

    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let final_output = {
        let export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return Err("export was cancelled".into());
        }
        export
            .final_output
            .clone()
            .ok_or_else(|| "missing final export path".to_string())?
    };

    let video_output = if mode == RenderExportMode::VideoAudio {
        Some(next_export_temp_path(&final_output, "video", "mp4"))
    } else if mode.wants_video() {
        Some(final_output.clone())
    } else {
        None
    };
    let audio_output = if mode == RenderExportMode::VideoAudio {
        Some(next_export_temp_path(&final_output, "audio", "wav"))
    } else if mode == RenderExportMode::AudioOnly {
        Some(if audio_format == AudioOnlyFormat::Wav {
            final_output.clone()
        } else {
            next_export_temp_path(&final_output, "audio", "wav")
        })
    } else {
        None
    };
    let finalize_spec = match mode {
        RenderExportMode::VideoAudio => {
            let mut extra_args = vec![];
            let bitrate = app.get_render_audio_bitrate_text();
            if !bitrate.is_empty() {
                extra_args.extend(["-b:a".to_string(), bitrate.to_string()]);
            }
            extra_args.extend(shell_words(
                app.get_render_audio_ffmpeg_args_text().as_str(),
            ));
            Some(FinalizeSpec::MuxMp4 {
                video_input: video_output
                    .clone()
                    .expect("video+audio export must have a temporary video output"),
                audio_input: audio_output
                    .clone()
                    .expect("video+audio export must have a temporary audio output"),
                extra_args,
            })
        }
        RenderExportMode::AudioOnly if audio_format != AudioOnlyFormat::Wav => {
            let extra_args = shell_words(app.get_render_audio_ffmpeg_args_text().as_str());
            Some(FinalizeSpec::EncodeAudio {
                wav_input: audio_output
                    .clone()
                    .expect("encoded audio export must render a temporary wav"),
                format: audio_format,
                extra_args,
            })
        }
        _ => None,
    };

    let video_config = video_output
        .clone()
        .map(|output| build_video_render_config(app, &snapshot, output))
        .transpose()?;
    let audio_config = audio_output
        .clone()
        .map(|output| build_audio_render_config(app, &snapshot, output))
        .transpose()?;

    {
        let mut export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return Err("export was cancelled".into());
        }
        export.mode = Some(mode);
        export.video_job_output = video_output;
        export.audio_job_output = audio_output;
        export.video_outcome = None;
        export.audio_outcome = None;
        export.finalize_spec = finalize_spec;
        export.finalizing = false;
        export.finalize_result = None;
    }

    if let Some(config) = video_config {
        let events = bridge
            .start_render_video(config, shared_state)
            .map_err(|error| error.to_string())?;
        if let Some(message) = events_error_message(&events) {
            return Err(message);
        }
        apply_events_to_app(app, shared_state, &events);
    }

    if let Some(config) = audio_config {
        let events = bridge
            .start_render_audio(config, shared_state)
            .map_err(|error| error.to_string())?;
        if let Some(message) = events_error_message(&events) {
            if app.get_video_render_status() != "Idle" {
                if let Ok(cancel_events) = bridge.cancel_render_video(shared_state) {
                    apply_events_to_app(app, shared_state, &cancel_events);
                }
            }
            return Err(message);
        }
        apply_events_to_app(app, shared_state, &events);
    }

    let status = match mode {
        RenderExportMode::VideoAudio => "Rendering video + audio",
        RenderExportMode::VideoOnly => "Rendering video",
        RenderExportMode::AudioOnly => "Rendering audio",
    };
    set_export_status(app, status, final_output.display().to_string(), 0.0);
    app.window().request_redraw();
    Ok(())
}

fn wire_render_export_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) {
    {
        let app_weak = app.as_weak();
        app.on_select_render_mode(move |mode| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = match mode.as_str() {
                "video_only" => "video_only",
                "audio_only" => "audio_only",
                _ => "video_audio",
            };
            app.set_render_mode_text(mode.into());
            sync_render_output_path(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_resolution(move |preset| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_resolution_text(preset.clone());
            if let Ok((width, height)) = parse_render_resolution(preset.as_str()) {
                app.set_render_video_width_text(width.to_string().into());
                app.set_render_video_height_text(height.to_string().into());
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_resolution(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            // Accept "WxH" or "W×H" or "W H"
            let normalized = val.replace('×', "x").replace(' ', "x");
            if parse_render_resolution(&normalized).is_ok() {
                app.set_render_video_resolution_text(normalized.into());
                if let Ok((w, h)) =
                    parse_render_resolution(app.get_render_video_resolution_text().as_str())
                {
                    app.set_render_video_width_text(w.to_string().into());
                    app.set_render_video_height_text(h.to_string().into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_fps(move |fps| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_fps_text(fps);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_fps(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            if parse_render_fps(val.as_str()).is_ok() {
                app.set_render_video_fps_text(val);
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_change_render_video_ffmpeg_args(move |args| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_ffmpeg_args_text(args);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_change_render_audio_ffmpeg_args(move |args| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_ffmpeg_args_text(args);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_codec(move |codec| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_codec_text(codec);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_crf(move |crf| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_crf_text(crf);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_video_crf(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            // Validate CRF is a number 0-51
            if let Ok(n) = val.trim().parse::<u32>() {
                if n <= 51 {
                    app.set_render_video_crf_text(n.to_string().into());
                }
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_preset(move |preset| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_preset_text(preset);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_video_pix_fmt(move |fmt| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_video_pix_fmt_text(fmt);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_bitrate(move |rate| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_bitrate_text(rate);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_submit_custom_render_audio_bitrate(move |val| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                app.set_render_audio_bitrate_text(trimmed.into());
            }
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_toggle_render_use_limiter(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_use_limiter(!app.get_render_use_limiter());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_toggle_render_open_after_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_open_after_export(!app.get_render_open_after_export());
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_format(move |format| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let format = match format.as_str() {
                "flac" => "flac",
                "mp3" => "mp3",
                _ => "wav",
            };
            app.set_render_audio_format_text(format.into());
            sync_render_output_path(&app);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_sample_rate(move |rate| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_sample_rate_text(rate);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_select_render_audio_channel_count(move |channels| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            app.set_render_audio_channel_count_text(channels);
            app.window().request_redraw();
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_browse_render_output(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let app_weak = app_weak.clone();
            let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
            let audio_format =
                AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
            let suggested_output = current_export_output_path(&app).unwrap_or_else(|_| {
                PathBuf::from(default_render_output_path(
                    app.get_selected_midi_name().as_str(),
                    mode,
                    audio_format,
                ))
            });
            std::thread::spawn(move || {
                let mut dialog = rfd::FileDialog::new();
                if let Some(name) = suggested_output.file_name().and_then(OsStr::to_str) {
                    dialog = dialog.set_file_name(name);
                }
                dialog = match mode {
                    RenderExportMode::VideoAudio | RenderExportMode::VideoOnly => {
                        dialog.add_filter("MP4", &["mp4"])
                    }
                    RenderExportMode::AudioOnly => match audio_format {
                        AudioOnlyFormat::Wav => dialog.add_filter("WAV", &["wav"]),
                        AudioOnlyFormat::Flac => dialog.add_filter("FLAC", &["flac"]),
                        AudioOnlyFormat::Mp3 => dialog.add_filter("MP3", &["mp3"]),
                    },
                };
                let Some(path) = dialog.save_file() else {
                    return;
                };
                let _ = app_weak.upgrade_in_event_loop(move |app| {
                    let normalized = normalize_output_path(&path, mode, audio_format);
                    app.set_render_output_path_text(normalized.display().to_string().into());
                    app.window().request_redraw();
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let export_state = Arc::clone(export_state);
        app.on_start_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
            let final_output = match current_export_output_path(&app) {
                Ok(path) => path,
                Err(message) => {
                    set_export_status(&app, "Failed", message, 0.0);
                    app.window().request_redraw();
                    return;
                }
            };

            {
                let mut export = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                if export.active {
                    return;
                }
                *export = RenderExportCoordinator {
                    active: true,
                    mode: Some(mode),
                    final_output: Some(final_output.clone()),
                    ..RenderExportCoordinator::default()
                };
            }

            app.set_render_output_path_text(final_output.display().to_string().into());

            if mode.wants_audio() {
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    fail_export(
                        &app,
                        &export_state,
                        "audio cache is already loading; wait for it to finish first".into(),
                    );
                    return;
                }
                if app.get_audio_load_state() != MidiLoadState::Loaded {
                    let midi_path = shared_state
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.midi_path.clone())
                        .or_else(|| {
                            let selected = app.get_selected_midi_name();
                            (!selected.is_empty()).then(|| PathBuf::from(selected.as_str()))
                        });
                    let Some(midi_path) = midi_path else {
                        fail_export(&app, &export_state, "load a MIDI before exporting".into());
                        return;
                    };

                    app.set_audio_load_state(MidiLoadState::Loading);
                    app.set_audio_loading_progress(0.0);
                    app.set_audio_loading_status("Preparing audio cache…".into());
                    app.set_audio_load_error(Default::default());
                    set_export_status(
                        &app,
                        "Preparing audio cache",
                        final_output.display().to_string(),
                        0.0,
                    );
                    app.window().request_redraw();

                    let app_weak = app.as_weak();
                    let bridge = bridge.clone();
                    let shared_state = Arc::clone(&shared_state);
                    let export_state = Arc::clone(&export_state);
                    std::thread::spawn(move || {
                        let result = bridge.load_audio_midi(midi_path, &shared_state);
                        let _ = app_weak.upgrade_in_event_loop(move |app| match result {
                            Ok(events) => {
                                if let Some(message) = events_error_message(&events) {
                                    app.set_audio_load_state(MidiLoadState::Error);
                                    app.set_audio_load_error(message.clone().into());
                                    app.set_audio_loading_status(Default::default());
                                    fail_export(&app, &export_state, message);
                                    return;
                                }
                                apply_events_to_app(&app, &shared_state, &events);
                                app.set_audio_load_state(MidiLoadState::Loaded);
                                app.set_audio_loading_progress(1.0);
                                app.set_audio_loading_status("Audio ready".into());
                                if export_state
                                    .lock()
                                    .expect("render export coordinator mutex poisoned")
                                    .active
                                {
                                    if let Err(message) = start_render_export_jobs(
                                        &app,
                                        &bridge,
                                        &shared_state,
                                        &export_state,
                                    ) {
                                        fail_export(&app, &export_state, message);
                                    }
                                }
                            }
                            Err(error) => {
                                app.set_audio_load_state(MidiLoadState::Error);
                                app.set_audio_load_error(error.to_string().into());
                                app.set_audio_loading_status(Default::default());
                                fail_export(&app, &export_state, error.to_string());
                            }
                        });
                    });
                    return;
                }
            }

            set_export_status(
                &app,
                "Preparing export",
                final_output.display().to_string(),
                0.0,
            );
            if let Err(message) =
                start_render_export_jobs(&app, &bridge, &shared_state, &export_state)
            {
                fail_export(&app, &export_state, message);
            }
        });
    }
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let export_state = Arc::clone(export_state);
        app.on_cancel_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let (active, finalizing) = {
                let export = export_state
                    .lock()
                    .expect("render export coordinator mutex poisoned");
                (export.active, export.finalizing)
            };
            if !active {
                return;
            }

            if finalizing {
                set_export_status(
                    &app,
                    "Finalizing output",
                    "Cancel is unavailable during final mux/encode".to_string(),
                    app.get_render_export_progress(),
                );
                app.window().request_redraw();
                return;
            }

            let video_running = app.get_video_render_status() != "Idle";
            let audio_running = app.get_audio_render_status() != "Idle";
            if video_running {
                if let Ok(events) = bridge.cancel_render_video(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
            }
            if audio_running {
                if let Ok(events) = bridge.cancel_render_audio(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
            }

            if !video_running && !audio_running {
                clear_export_state(&export_state);
                set_export_status(&app, "Cancelled", "Render cancelled".to_string(), 0.0);
            }
            app.window().request_redraw();
        });
    }
}

/// Wire MIDI file selection, loading, unloading, and drag-drop callbacks.
fn wire_midi_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    // ── Browse button (open native file dialog) ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_select_midi_file(move || {
            let app_weak = app_weak.clone();
            let bridge = bridge.clone();
            let shared_state = Arc::clone(&shared_state);
            let preview_load_generation = Arc::clone(&preview_load_generation);
            let render_load_generation = Arc::clone(&render_load_generation);
            let audio_load_generation = Arc::clone(&audio_load_generation);
            let analysis_load_generation = Arc::clone(&analysis_load_generation);
            // rfd's async dialog won't block the event loop on supported platforms
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let name: slint::SharedString = path.display().to_string().into();
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        replace_selected_midi(
                            &app,
                            &bridge,
                            &shared_state,
                            &preview_load_generation,
                            &render_load_generation,
                            &audio_load_generation,
                            &analysis_load_generation,
                            name,
                        );
                    });
                }
            });
        });
    }

    // ── File dropped from OS ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_drop_midi_file(move |path| {
            if let Some(app) = app_weak.upgrade() {
                if app.get_active_profile() == 6 {
                    append_merge_source_paths(
                        &app,
                        &bridge,
                        &shared_state,
                        vec![PathBuf::from(path.as_str())],
                    );
                    return;
                }
                replace_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                    path.clone(),
                );
                // Auto-load for the current profile context
                match app.get_active_profile() {
                    0 => load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &path,
                    ),
                    1 | 2 => load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &path,
                    ),
                    3 => load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &path,
                    ),
                    _ => load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &path,
                    ),
                }
            }
        });
    }

    // ── Load for preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_load_for_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_load_for_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_load_for_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for analysis / modify ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_load_for_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Cancel preview load ──
    {
        let app_weak = app.as_weak();
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_cancel_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                preview_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    // ── Cancel render load ──
    {
        let app_weak = app.as_weak();
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_cancel_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
            }
        });
    }

    // ── Cancel analysis load ──
    {
        let app_weak = app.as_weak();
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_cancel_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_analysis_load_state() == MidiLoadState::Loading {
                    app.set_analysis_load_state(MidiLoadState::Selected);
                }
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
            }
        });
    }

    // ── Cancel audio load ──
    {
        let app_weak = app.as_weak();
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_cancel_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    // ── Unload preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    // ── Unload render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_render(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    // ── Unload audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    // ── Unload analysis ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                unload_selected_midi(
                    &app,
                    &bridge,
                    &shared_state,
                    &preview_load_generation,
                    &render_load_generation,
                    &audio_load_generation,
                    &analysis_load_generation,
                );
            }
        });
    }

    // ── Retry load preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_retry_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_retry_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_retry_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load analysis ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_retry_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }
}

/// Start a preview MIDI load without blocking the Slint event loop.
fn load_preview_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Loading preview playback…".into());
    app.set_audio_loading_status("Loading preview playback…".into());
    app.set_render_loading_progress(0.0);
    app.set_audio_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = preview_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let preview_load_generation = Arc::clone(preview_load_generation);
    std::thread::spawn(move || {
        let result = (|| -> Result<Vec<CoreEvent>, MeridianError> {
            let mut events = bridge.load_midi(path, &shared_state)?;
            events.extend(bridge.set_playing(true, &shared_state)?);
            Ok(events)
        })();
        if preview_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if preview_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_audio_loading_progress(1.0);
                    app.set_render_loading_status("Preview ready".into());
                    app.set_audio_loading_status("Preview ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_audio_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                    app.set_audio_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

/// Start a render-context MIDI load without blocking the Slint event loop.
fn load_render_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    render_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Preparing display cache…".into());
    app.set_render_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = render_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let render_load_generation = Arc::clone(render_load_generation);
    std::thread::spawn(move || {
        let result = bridge.load_display_midi(path, &shared_state);
        if render_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if render_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_render_loading_status("Render visuals ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_audio_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    audio_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_audio_loading_status("Preparing audio cache…".into());
    app.set_audio_loading_progress(0.0);
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = audio_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let audio_load_generation = Arc::clone(audio_load_generation);
    std::thread::spawn(move || {
        let result = bridge.load_audio_midi(path, &shared_state);
        if audio_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if audio_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_audio_loading_progress(1.0);
                    app.set_audio_loading_status("Audio ready".into());
                }
                Err(e) => {
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_audio_load_error(e.to_string().into());
                    app.set_audio_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    analysis_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_analysis_load_state(MidiLoadState::Loading);
    app.set_analysis_loading_status("Reading MIDI for analysis…".into());
    app.set_analysis_loading_progress(0.0);
    app.set_analysis_load_error(Default::default());
    reset_analysis_outputs(app);
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = analysis_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    std::thread::spawn(move || {
        let progress = |value: f32, status: &'static str| {
            update_analysis_progress(
                &app_weak,
                &requested_name,
                &analysis_load_generation,
                request_generation,
                value,
                status,
            );
        };
        progress(0.15, "Parsing MIDI for analysis…");
        let result = load_analysis_resource_set(&bridge, &shared_state, &path, progress);
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_analysis_load_state(MidiLoadState::Loaded);
                    app.set_analysis_loading_progress(1.0);
                    app.set_analysis_loading_status("Analysis ready".into());
                }
                Err(e) => {
                    app.set_analysis_load_state(MidiLoadState::Error);
                    app.set_analysis_load_error(e.to_string().into());
                    app.set_analysis_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_resource_set(
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    path: &std::path::Path,
    mut progress: impl FnMut(f32, &'static str),
) -> Result<Vec<CoreEvent>, MeridianError> {
    let mut all_events = Vec::new();

    let parsed_events = bridge.load_parsed_midi(path.to_path_buf(), shared_state)?;
    let parsed_midi_id = parsed_midi_id_from_events(&parsed_events)?;
    all_events.extend(parsed_events);

    progress(0.55, "Building analysis model…");
    let processed_events = bridge.build_processed_midi(
        parsed_midi_id,
        MidiProcessingConfig::default(),
        shared_state,
    )?;
    let (processed_midi_id, midi_length) = processed_result_from_events(&processed_events)?;
    all_events.extend(processed_events);

    let bucket_count = ((midi_length / 0.5).ceil() as usize).clamp(1, 8192);
    progress(0.82, "Computing bucketed note statistics…");
    let analysis_events =
        bridge.analyze_processed_midi(processed_midi_id, Some(bucket_count), shared_state)?;
    all_events.extend(analysis_events);

    Ok(all_events)
}

fn parsed_midi_id_from_events(events: &[CoreEvent]) -> Result<ParsedMidiId, MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing parsed MIDI id".into()))
}

fn processed_result_from_events(
    events: &[CoreEvent],
) -> Result<(ProcessedMidiId, f64), MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ProcessedMidiBuilt {
                processed_midi_id,
                midi_length,
                ..
            } => Some((*processed_midi_id, *midi_length)),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing processed MIDI id".into()))
}

fn reset_analysis_outputs(app: &App) {
    app.set_analysis_note_count_text("—".into());
    app.set_analysis_track_count_text("—".into());
    app.set_analysis_midi_length_text("—".into());
    app.set_analysis_tempo_text("—".into());
    app.set_analysis_time_signature_text("—".into());
    app.set_analysis_key_range_text("—".into());
    app.set_analysis_avg_velocity_text("—".into());
    app.set_analysis_note_density_text("—".into());
}

fn update_analysis_progress(
    app_weak: &slint::Weak<App>,
    requested_name: &slint::SharedString,
    analysis_load_generation: &Arc<AtomicU64>,
    request_generation: u64,
    progress: f32,
    status: &str,
) {
    let app_weak = app_weak.clone();
    let requested_name = requested_name.clone();
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    let status_text: slint::SharedString = status.into();
    let _ = app_weak.upgrade_in_event_loop(move |app| {
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        if app.get_selected_midi_name() != requested_name {
            return;
        }
        if app.get_analysis_load_state() != MidiLoadState::Loading {
            return;
        }
        app.set_analysis_loading_progress(progress);
        app.set_analysis_loading_status(status_text);
        app.window().request_redraw();
    });
}

/// Install the winit window event handler for OS file drag-and-drop.
fn install_drag_drop(app: &App) {
    let app_weak = app.as_weak();
    app.window()
        .on_winit_window_event(move |_slint_window, event| match event {
            winit::event::WindowEvent::HoveredFile(_path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(true);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::DroppedFile(path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    let path_str: slint::SharedString = path.display().to_string().into();
                    app.invoke_drop_midi_file(path_str);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::HoveredFileCancelled => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            _ => EventResult::Propagate,
        });
}

fn install_core_event_listener(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    let shared_state = Arc::clone(shared_state);
    let export_state = Arc::clone(export_state);
    std::thread::spawn(move || {
        for event in receiver {
            update_export_state_from_event(&export_state, &event);
            match &event {
                CoreEvent::MidiLoadProgress {
                    path,
                    progress,
                    status,
                } => {
                    let path_text: slint::SharedString = path.display().to_string().into();
                    let status_text: slint::SharedString = status.clone().into();
                    let progress = *progress;
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        if app.get_selected_midi_name() != path_text {
                            return;
                        }
                        if app.get_render_load_state() == MidiLoadState::Loading {
                            if let Some(progress) = progress {
                                app.set_render_loading_progress(progress);
                            }
                            app.set_render_loading_status(status_text.clone());
                        }
                        if app.get_audio_load_state() == MidiLoadState::Loading {
                            if let Some(progress) = progress {
                                app.set_audio_loading_progress(progress);
                            }
                            app.set_audio_loading_status(status_text);
                        }
                        app.window().request_redraw();
                    });
                }
                CoreEvent::AudioRender { .. }
                | CoreEvent::AudioRenderStatus { .. }
                | CoreEvent::VideoRender { .. }
                | CoreEvent::VideoRenderStatus { .. }
                | CoreEvent::Error { .. } => {
                    let event = event.clone();
                    let shared_state = Arc::clone(&shared_state);
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        shared_state
                            .lock()
                            .expect("shared UI state mutex poisoned")
                            .reduce_events(std::slice::from_ref(&event));
                        apply_events_to_app(&app, &shared_state, &[event]);
                        app.window().request_redraw();
                    });
                }
                _ => {}
            }
        }
    });
}

fn update_export_state_from_event(
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
    event: &CoreEvent,
) {
    let mut export = export_state
        .lock()
        .expect("render export coordinator mutex poisoned");
    if !export.active {
        return;
    }

    match event {
        CoreEvent::VideoRender { event } => match event {
            meridian_core::protocol::VideoRenderEvent::RenderFinished { output, .. } => {
                if export.video_job_output.as_ref() == Some(output) {
                    export.video_outcome = Some(RenderJobOutcome::Finished);
                }
            }
            meridian_core::protocol::VideoRenderEvent::RenderCancelled { .. } => {
                if export.mode.is_some_and(RenderExportMode::wants_video) {
                    export.video_outcome = Some(RenderJobOutcome::Cancelled);
                }
            }
            meridian_core::protocol::VideoRenderEvent::RenderFailed { message } => {
                if export.mode.is_some_and(RenderExportMode::wants_video) {
                    export.video_outcome = Some(RenderJobOutcome::Failed(message.clone()));
                }
            }
            _ => {}
        },
        CoreEvent::AudioRender { event } => match event {
            meridian_core::audio::AudioRenderEvent::RenderFinished { output, .. } => {
                if export.audio_job_output.as_ref() == Some(output) {
                    export.audio_outcome = Some(RenderJobOutcome::Finished);
                }
            }
            meridian_core::audio::AudioRenderEvent::RenderCancelled { .. } => {
                if export.mode.is_some_and(RenderExportMode::wants_audio) {
                    export.audio_outcome = Some(RenderJobOutcome::Cancelled);
                }
            }
            meridian_core::audio::AudioRenderEvent::RenderFailed { message } => {
                if export.mode.is_some_and(RenderExportMode::wants_audio) {
                    export.audio_outcome = Some(RenderJobOutcome::Failed(message.clone()));
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn spawn_finalizer_task(
    export_state: Arc<Mutex<RenderExportCoordinator>>,
    spec: FinalizeSpec,
    final_output: PathBuf,
) {
    std::thread::spawn(move || {
        let result = run_finalizer(&spec, &final_output);
        match &spec {
            FinalizeSpec::MuxMp4 {
                video_input,
                audio_input,
                ..
            } => {
                let _ = fs::remove_file(video_input);
                let _ = fs::remove_file(audio_input);
            }
            FinalizeSpec::EncodeAudio { wav_input, .. } => {
                let _ = fs::remove_file(wav_input);
            }
        }
        export_state
            .lock()
            .expect("render export coordinator mutex poisoned")
            .finalize_result = Some(result);
    });
}

fn update_render_export_ui(app: &App, export_state: &Arc<Mutex<RenderExportCoordinator>>) {
    enum UiAction {
        None,
        Finalize { spec: FinalizeSpec, output: PathBuf },
        Finished(PathBuf),
        Failed(String),
        Cancelled,
    }

    let action = {
        let mut export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return;
        }

        if export.finalizing {
            if let Some(result) = export.finalize_result.take() {
                export.active = false;
                export.finalizing = false;
                export.mode = None;
                match result {
                    Ok(()) => UiAction::Finished(
                        export
                            .final_output
                            .clone()
                            .unwrap_or_else(|| PathBuf::from("(output)")),
                    ),
                    Err(message) => UiAction::Failed(message),
                }
            } else {
                set_export_status(
                    app,
                    "Finalizing output",
                    export
                        .final_output
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "encoding final output".into()),
                    0.98,
                );
                UiAction::None
            }
        } else {
            let Some(mode) = export.mode else {
                return;
            };

            if let Some(message) = match &export.video_outcome {
                Some(RenderJobOutcome::Failed(message)) => Some(message.clone()),
                _ => None,
            } {
                export.active = false;
                UiAction::Failed(message)
            } else if let Some(message) = match &export.audio_outcome {
                Some(RenderJobOutcome::Failed(message)) => Some(message.clone()),
                _ => None,
            } {
                export.active = false;
                UiAction::Failed(message)
            } else if matches!(export.video_outcome, Some(RenderJobOutcome::Cancelled))
                || matches!(export.audio_outcome, Some(RenderJobOutcome::Cancelled))
            {
                export.active = false;
                UiAction::Cancelled
            } else {
                let video_done = !mode.wants_video()
                    || matches!(export.video_outcome, Some(RenderJobOutcome::Finished));
                let audio_done = !mode.wants_audio()
                    || matches!(export.audio_outcome, Some(RenderJobOutcome::Finished));

                if video_done && audio_done {
                    if let Some(spec) = export.finalize_spec.clone() {
                        let output = export
                            .final_output
                            .clone()
                            .unwrap_or_else(|| PathBuf::from("render.out"));
                        export.finalizing = true;
                        UiAction::Finalize { spec, output }
                    } else {
                        export.active = false;
                        UiAction::Finished(
                            export
                                .final_output
                                .clone()
                                .unwrap_or_else(|| PathBuf::from("render.out")),
                        )
                    }
                } else {
                    let progress = match mode {
                        RenderExportMode::VideoAudio => {
                            (app.get_video_render_progress() + app.get_audio_render_progress())
                                / 2.0
                        }
                        RenderExportMode::VideoOnly => app.get_video_render_progress(),
                        RenderExportMode::AudioOnly => app.get_audio_render_progress(),
                    };
                    let status = match mode {
                        RenderExportMode::VideoAudio => "Rendering video + audio",
                        RenderExportMode::VideoOnly => "Rendering video",
                        RenderExportMode::AudioOnly => "Rendering audio",
                    };
                    set_export_status(
                        app,
                        status,
                        export
                            .final_output
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "working".into()),
                        progress,
                    );
                    UiAction::None
                }
            }
        }
    };

    match action {
        UiAction::None => {}
        UiAction::Finalize { spec, output } => {
            set_export_status(app, "Finalizing output", output.display().to_string(), 0.98);
            spawn_finalizer_task(Arc::clone(export_state), spec, output);
        }
        UiAction::Finished(output) => {
            set_export_status(app, "Finished", output.display().to_string(), 1.0);
            if app.get_render_open_after_export() {
                let _ = open::that_detached(&output);
            }
        }
        UiAction::Failed(message) => {
            set_export_status(app, "Failed", message, 0.0);
        }
        UiAction::Cancelled => {
            set_export_status(app, "Cancelled", "Render cancelled".to_string(), 0.0);
        }
    }
}

fn install_viewport(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> Result<(), MeridianError> {
    if disable_wgpu {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
        return Ok(());
    }

    let renderer = std::rc::Rc::new(std::cell::RefCell::new(ViewportRenderer::new(
        app.as_weak(),
        core.clone(),
        Arc::clone(shared_state),
        Rc::clone(pending_viewport_image),
        Rc::clone(viewport_size),
        true,
    )));
    let renderer_for_notifier = std::rc::Rc::clone(&renderer);
    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            renderer_for_notifier
                .borrow_mut()
                .handle(state, graphics_api);
        })
        .map_err(|e| MeridianError::SlintNotifier(e.to_string()))
}

fn install_timer(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) -> slint::Timer {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let bridge_for_timer = bridge.clone();
    let shared_state_for_timer = Arc::clone(shared_state);
    let pending_viewport_image_for_timer = Rc::clone(pending_viewport_image);
    let viewport_size_for_timer = Rc::clone(viewport_size);
    let export_state_for_timer = Arc::clone(export_state);

    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(app) = app_for_timer.upgrade() {
                *viewport_size_for_timer.borrow_mut() = (
                    app.get_viewport_px_width().max(1.0) as u32,
                    app.get_viewport_px_height().max(1.0) as u32,
                );
                if let Some(image) = pending_viewport_image_for_timer.borrow_mut().take() {
                    app.set_viewport_image(image);
                }
                if !app_is_loading(&app) {
                    if let Ok(events) = bridge_for_timer.refresh_state(&shared_state_for_timer) {
                        apply_events_to_app(&app, &shared_state_for_timer, &events);
                        let playing = shared_state_for_timer
                            .lock()
                            .expect("shared UI state mutex poisoned")
                            .transport
                            .playing;
                        if playing || disable_wgpu {
                            app.window().request_redraw();
                        }
                    }
                }
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
                update_render_export_ui(&app, &export_state_for_timer);
            }
        },
    );
    animation_timer
}
