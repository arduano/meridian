use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use meridian_core::{
    MeridianError,
    midi::{
        MidiFileProcessingConfig, QuantizeMode, QuantizeTool, RangeEdgeBehavior, RangeSelectTool,
        TempoMapTool, analysis::MidiAnalysisKind,
    },
    protocol::{
        MidiAnalysisJobStatus, MidiProcessEvent, MidiProcessStatus, ProtocolClient,
        ProtocolCommand, ProtocolEvent,
    },
    render::{DisplayTimeSpace, RendererKind},
};

#[derive(Debug, Parser)]
#[command(name = "meridian")]
#[command(version)]
#[command(about = "Meridian command line interface")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(visible_alias = "serve-json")]
    Stdio,
    Json(JsonArgs),
    Analyze(AnalyzeArgs),
    Process {
        #[command(subcommand)]
        command: ProcessCommand,
    },
    Render {
        #[command(subcommand)]
        command: RenderCommand,
    },
    Bench(BenchmarkArgs),
    Debug {
        #[command(subcommand)]
        command: DebugCommand,
    },
    #[command(hide = true)]
    FrameStdout(FrameStdoutArgs),
    #[command(hide = true)]
    Benchmark(BenchmarkArgs),
    #[command(hide = true)]
    RenderVideo(RenderVideoArgs),
    #[command(hide = true)]
    RenderAudio(RenderAudioArgs),
    #[command(hide = true)]
    DebugPianoTrailClassicGeometry(DebugPianoTrailClassicGeometryArgs),
}

pub fn run() -> Result<(), MeridianError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Stdio => crate::json_mode::serve_json(),
        Command::Json(args) => crate::json_mode::run_one_json(args.raw.join(" ").trim()),
        Command::Analyze(args) => run_analyze(args),
        Command::Process {
            command: ProcessCommand::Select(args),
        } => run_process(
            ProcessTool::Select(process_range_select_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::TempoFlatten(args),
        } => run_process(
            ProcessTool::Tempo(process_tempo_flatten_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::TempoScale(args),
        } => run_process(
            ProcessTool::Tempo(process_tempo_scale_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::Quantize(args),
        } => run_process(
            ProcessTool::Quantize(process_quantize_tool(&args)),
            args.common,
        ),
        Command::Render {
            command: RenderCommand::Frame(args),
        }
        | Command::FrameStdout(args) => run_frame_stdout(args),
        Command::Render {
            command: RenderCommand::Video(args),
        }
        | Command::RenderVideo(args) => run_render_video(args),
        Command::Render {
            command: RenderCommand::Audio(args),
        }
        | Command::RenderAudio(args) => run_render_audio(args),
        Command::Bench(args) | Command::Benchmark(args) => run_benchmark(args),
        Command::Debug {
            command: DebugCommand::PianoTrailClassicGeometry(args),
        }
        | Command::DebugPianoTrailClassicGeometry(args) => run_debug_geometry(args),
    }
}

#[derive(Debug, Args)]
pub struct JsonArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    raw: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    #[arg(value_name = "MIDI")]
    midi: PathBuf,
    #[arg(long = "include", value_enum)]
    include: Vec<AnalysisKindArg>,
    #[arg(long)]
    buckets: Option<usize>,
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Subcommand)]
pub enum ProcessCommand {
    Select(ProcessSelectArgs),
    TempoFlatten(ProcessTempoFlattenArgs),
    TempoScale(ProcessTempoScaleArgs),
    Quantize(ProcessQuantizeArgs),
}

#[derive(Debug, Subcommand)]
pub enum RenderCommand {
    Frame(FrameStdoutArgs),
    Video(RenderVideoArgs),
    Audio(RenderAudioArgs),
}

#[derive(Debug, Subcommand)]
pub enum DebugCommand {
    PianoTrailClassicGeometry(DebugPianoTrailClassicGeometryArgs),
}

#[derive(Debug, Clone, Args)]
pub struct FrameStdoutArgs {
    #[arg(value_name = "MIDI")]
    midi: PathBuf,
    #[arg(long, value_enum, default_value_t = meridian_core::protocol::ImageOutputFormat::Rgba)]
    format: meridian_core::protocol::ImageOutputFormat,
    #[arg(long, default_value_t = 0.0)]
    time: f64,
    #[arg(long, default_value_t = 8.0)]
    view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    first_key: u8,
    #[arg(long, default_value_t = 127)]
    last_key: u8,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    renderer: RendererKind,
}

#[derive(Debug, Clone, Args)]
pub struct BenchmarkArgs {
    #[arg(value_name = "MIDI")]
    midi: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    time: f64,
    #[arg(long, default_value_t = 8.0)]
    view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    first_key: u8,
    #[arg(long, default_value_t = 127)]
    last_key: u8,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    renderer: RendererKind,
    #[arg(long, default_value_t = 120)]
    iterations: u32,
    #[arg(long, default_value_t = 10)]
    warmup: u32,
}

#[derive(Debug, Clone, Args)]
pub struct RenderVideoArgs {
    #[arg(value_name = "MIDI")]
    midi: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    fps: f64,
    #[arg(long, default_value_t = 1920)]
    width: u32,
    #[arg(long, default_value_t = 1080)]
    height: u32,
    #[arg(long, default_value_t = 8.0)]
    view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    first_key: u8,
    #[arg(long, default_value_t = 127)]
    last_key: u8,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    renderer: RendererKind,
    #[arg(long, allow_hyphen_values = true)]
    ffmpeg_flags: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct RenderAudioArgs {
    #[arg(value_name = "MIDI")]
    midi: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 44_100)]
    sample_rate: u32,
    #[arg(long, default_value_t = 2)]
    channels: u16,
    #[arg(long)]
    no_limiter: bool,
    #[arg(long)]
    soundfont: Vec<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct DebugPianoTrailClassicGeometryArgs {
    #[arg(long, default_value_t = 0)]
    first_key: u8,
    #[arg(long, default_value_t = 127)]
    last_key: u8,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
    #[arg(long, allow_hyphen_values = true)]
    scene_json: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ProcessCommonArgs {
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ProcessSelectArgs {
    #[command(flatten)]
    common: ProcessCommonArgs,
    #[arg(long)]
    start_ticks: u64,
    #[arg(long)]
    end_ticks: u64,
    #[arg(long)]
    offset_ticks: Option<u64>,
    #[arg(long)]
    track_select: Option<usize>,
    #[arg(long)]
    preserve_system_events: bool,
    #[arg(long, value_enum, default_value_t = RangeEdgeBehaviorArg::Trim)]
    edge_behavior: RangeEdgeBehaviorArg,
}

#[derive(Debug, Clone, Args)]
pub struct ProcessTempoFlattenArgs {
    #[command(flatten)]
    common: ProcessCommonArgs,
    #[arg(long)]
    tempo: u32,
}

#[derive(Debug, Clone, Args)]
pub struct ProcessTempoScaleArgs {
    #[command(flatten)]
    common: ProcessCommonArgs,
    #[arg(long)]
    factor: f64,
}

#[derive(Debug, Clone, Args)]
pub struct ProcessQuantizeArgs {
    #[command(flatten)]
    common: ProcessCommonArgs,
    #[arg(long)]
    grid_ticks: u64,
    #[arg(long, value_enum, default_value_t = QuantizeModeArg::NoteStartOnly)]
    mode: QuantizeModeArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AnalysisKindArg {
    File,
    Summary,
    Events,
    Notes,
    Tempo,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RangeEdgeBehaviorArg {
    Keep,
    Skip,
    Trim,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum QuantizeModeArg {
    NoteStartOnly,
    NoteStartAndEnd,
    AllEvents,
}

enum ProcessTool {
    Select(RangeSelectTool),
    Tempo(TempoMapTool),
    Quantize(QuantizeTool),
}

fn run_analyze(args: AnalyzeArgs) -> Result<(), MeridianError> {
    let kinds = analysis_kinds(&args);
    let bucket_count = args.buckets;
    let client = ProtocolClient::spawn();

    let parsed_events = client.request(ProtocolCommand::LoadParsedMidi {
        path: args.midi.clone(),
    })?;
    let parsed_midi_id = parsed_events
        .events
        .iter()
        .find_map(|event| match event {
            ProtocolEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .ok_or_else(|| MeridianError::Protocol("missing parsed midi id".into()))?;

    let status_events = client.request(ProtocolCommand::StartMidiAnalysisJob {
        parsed_midi_id,
        display_cache_id: None,
        kinds,
        bucket_count,
    })?;
    let job_id = match status_events.events.as_slice() {
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Running { job_id, .. },
            },
        ] => *job_id,
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Finished { result, .. },
            },
        ] => {
            print_json_to_stdout(result, args.pretty)?;
            let _ = client.shutdown();
            return Ok(());
        }
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Failed { message, .. },
            },
        ] => return Err(MeridianError::Protocol(message.clone())),
        other => {
            return Err(MeridianError::Protocol(format!(
                "unexpected analysis start response: {other:?}"
            )));
        }
    };

    let result = loop {
        let event = client
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| MeridianError::Platform("timed out waiting for analysis job".into()))?;
        match event {
            ProtocolEvent::MidiAnalysisJob { event } => match event {
                meridian_core::protocol::MidiAnalysisJobEvent::Progress {
                    job_id: event_job_id,
                    progress,
                    status,
                } if event_job_id == job_id => {
                    eprintln!("analysis: {:>3.0}% {status}", progress as f64 * 100.0);
                }
                meridian_core::protocol::MidiAnalysisJobEvent::Finished {
                    job_id: event_job_id,
                    result,
                } if event_job_id == job_id => break Ok(result),
                meridian_core::protocol::MidiAnalysisJobEvent::Failed {
                    job_id: event_job_id,
                    message,
                } if event_job_id == job_id => break Err(MeridianError::Protocol(message)),
                _ => {}
            },
            _ => {}
        }
    };

    let _ = client.shutdown();
    print_json_to_stdout(&result?, args.pretty)
}

fn run_process(tool: ProcessTool, common: ProcessCommonArgs) -> Result<(), MeridianError> {
    let config = build_process_config(&common, tool);
    let client = ProtocolClient::spawn();
    let status_events = client.request(ProtocolCommand::StartProcessMidiFile {
        input: common.input.clone(),
        output: common.output.clone(),
        config,
    })?;

    let job_id = match status_events.events.as_slice() {
        [
            ProtocolEvent::MidiProcessStatus {
                status: MidiProcessStatus::Running { job_id, .. },
            },
        ] => {
            eprintln!("process: started job {job_id:?}");
            *job_id
        }
        [
            ProtocolEvent::MidiProcessStatus {
                status: MidiProcessStatus::Idle,
            },
        ] => {
            return Err(MeridianError::Protocol(
                "midi processing did not enter a running state".into(),
            ));
        }
        other => {
            return Err(MeridianError::Protocol(format!(
                "unexpected process start response: {other:?}"
            )));
        }
    };

    let result = loop {
        let event = client
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| MeridianError::Platform("timed out waiting for midi processing".into()))?;
        match event {
            ProtocolEvent::MidiProcess { event } => match event {
                MidiProcessEvent::ProcessStarted {
                    job_id: event_job_id,
                    input,
                    ..
                } if event_job_id == job_id => {
                    eprintln!("process: {}", input.display());
                }
                MidiProcessEvent::ProcessFinished {
                    job_id: event_job_id,
                    ..
                } if event_job_id == job_id => break Ok(event),
                MidiProcessEvent::ProcessFailed {
                    job_id: event_job_id,
                    message,
                    ..
                } if event_job_id == job_id => break Err(MeridianError::Protocol(message)),
                MidiProcessEvent::ProcessCancelled {
                    job_id: event_job_id,
                    ..
                } if event_job_id == job_id => {
                    break Err(MeridianError::Cancelled(
                        "midi processing was cancelled".into(),
                    ));
                }
                _ => {}
            },
            _ => {}
        }
    };

    let _ = client.shutdown();
    print_json_to_stdout(&result?, common.pretty)
}

fn analysis_kinds(args: &AnalyzeArgs) -> Vec<MidiAnalysisKind> {
    let mut kinds = if args.include.is_empty() {
        vec![
            MidiAnalysisKind::File,
            MidiAnalysisKind::Summary,
            MidiAnalysisKind::Events,
            MidiAnalysisKind::Notes,
            MidiAnalysisKind::Tempo,
        ]
    } else {
        args.include
            .iter()
            .map(|kind| match kind {
                AnalysisKindArg::File => MidiAnalysisKind::File,
                AnalysisKindArg::Summary => MidiAnalysisKind::Summary,
                AnalysisKindArg::Events => MidiAnalysisKind::Events,
                AnalysisKindArg::Notes => MidiAnalysisKind::Notes,
                AnalysisKindArg::Tempo => MidiAnalysisKind::Tempo,
            })
            .collect::<Vec<_>>()
    };
    if args.buckets.is_some() {
        kinds.push(MidiAnalysisKind::Buckets);
    }
    kinds
}

fn build_process_config(common: &ProcessCommonArgs, tool: ProcessTool) -> MidiFileProcessingConfig {
    let _ = common;
    MidiFileProcessingConfig {
        tool: match tool {
            ProcessTool::Select(tool) => meridian_core::midi::MidiModifierTool::RangeSelect(tool),
            ProcessTool::Tempo(tool) => meridian_core::midi::MidiModifierTool::TempoMap(tool),
            ProcessTool::Quantize(tool) => meridian_core::midi::MidiModifierTool::Quantize(tool),
        },
    }
}

fn process_range_select_tool(args: &ProcessSelectArgs) -> RangeSelectTool {
    RangeSelectTool {
        start_ticks: args.start_ticks,
        end_ticks: args.end_ticks,
        offset_ticks: args.offset_ticks,
        track_select: args.track_select,
        preserve_system_events: args.preserve_system_events,
        edge_behavior: match args.edge_behavior {
            RangeEdgeBehaviorArg::Keep => RangeEdgeBehavior::Keep,
            RangeEdgeBehaviorArg::Skip => RangeEdgeBehavior::Skip,
            RangeEdgeBehaviorArg::Trim => RangeEdgeBehavior::Trim,
        },
    }
}

fn process_tempo_flatten_tool(args: &ProcessTempoFlattenArgs) -> TempoMapTool {
    TempoMapTool::Flatten { tempo: args.tempo }
}

fn process_tempo_scale_tool(args: &ProcessTempoScaleArgs) -> TempoMapTool {
    TempoMapTool::ScaleBpm {
        factor: args.factor,
    }
}

fn process_quantize_tool(args: &ProcessQuantizeArgs) -> QuantizeTool {
    QuantizeTool {
        rounding_ticks: args.grid_ticks,
        mode: match args.mode {
            QuantizeModeArg::NoteStartOnly => QuantizeMode::NoteStartOnly,
            QuantizeModeArg::NoteStartAndEnd => QuantizeMode::NoteStartAndEnd,
            QuantizeModeArg::AllEvents => QuantizeMode::AllEvents,
        },
    }
}

fn print_json_to_stdout(value: &impl serde::Serialize, pretty: bool) -> Result<(), MeridianError> {
    let mut stdout = io::stdout().lock();
    if pretty {
        serde_json::to_writer_pretty(&mut stdout, value)
            .map_err(|error| MeridianError::Platform(error.to_string()))?;
    } else {
        serde_json::to_writer(&mut stdout, value)
            .map_err(|error| MeridianError::Platform(error.to_string()))?;
    }
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn run_frame_stdout(args: FrameStdoutArgs) -> Result<(), MeridianError> {
    crate::frame_stdout::run(
        &args.midi,
        args.format,
        args.time,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.renderer,
    )
}

fn run_benchmark(args: BenchmarkArgs) -> Result<(), MeridianError> {
    crate::benchmark::run(
        &args.midi,
        args.time,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.renderer,
        args.iterations,
        args.warmup,
    )
}

fn run_render_video(args: RenderVideoArgs) -> Result<(), MeridianError> {
    crate::render_video::run(
        &args.midi,
        &args.output,
        args.fps,
        args.width,
        args.height,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.renderer,
        args.ffmpeg_flags.as_deref(),
    )
}

fn run_render_audio(args: RenderAudioArgs) -> Result<(), MeridianError> {
    crate::render_audio::run(
        &args.midi,
        &args.output,
        args.sample_rate,
        args.channels,
        !args.no_limiter,
        &args.soundfont,
    )
}

fn run_debug_geometry(args: DebugPianoTrailClassicGeometryArgs) -> Result<(), MeridianError> {
    crate::debug_piano_trail_classic::run(
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.scene_json.as_deref(),
    )
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
    path.exists()
}
