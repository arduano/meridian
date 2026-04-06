use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use meridian_core::{
    midi::{QuantizeMode, RangeEdgeBehavior, analysis::MidiAnalysisKind},
    protocol::{FrameColorMode, ImageOutputFormat},
    render::{DisplayTimeSpace, RendererKind},
};

#[derive(Debug, Parser)]
#[command(name = "meridian")]
#[command(version)]
#[command(about = "Meridian command line interface")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
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

#[derive(Debug, Args)]
pub(super) struct JsonArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(super) raw: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct AnalyzeArgs {
    #[arg(value_name = "MIDI")]
    pub(super) midi: PathBuf,
    #[arg(long = "include", value_enum)]
    pub(super) include: Vec<AnalysisKindArg>,
    #[arg(long)]
    pub(super) buckets: Option<usize>,
    #[arg(long)]
    pub(super) pretty: bool,
}

#[derive(Debug, Subcommand)]
pub(super) enum ProcessCommand {
    Select(ProcessSelectArgs),
    TempoFlatten(ProcessTempoFlattenArgs),
    TempoScale(ProcessTempoScaleArgs),
    Quantize(ProcessQuantizeArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum RenderCommand {
    Frame(FrameStdoutArgs),
    Video(RenderVideoArgs),
    Audio(RenderAudioArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum DebugCommand {
    PianoTrailClassicGeometry(DebugPianoTrailClassicGeometryArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct FrameStdoutArgs {
    #[arg(value_name = "MIDI")]
    pub(super) midi: PathBuf,
    #[arg(long, value_enum, default_value_t = ImageOutputFormat::Rgba)]
    pub(super) format: ImageOutputFormat,
    #[arg(long, default_value_t = 0.0)]
    pub(super) time: f64,
    #[arg(long, default_value_t = 8.0)]
    pub(super) view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    pub(super) time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    pub(super) first_key: u8,
    #[arg(long, default_value_t = 127)]
    pub(super) last_key: u8,
    #[arg(long, default_value_t = 1280)]
    pub(super) width: u32,
    #[arg(long, default_value_t = 720)]
    pub(super) height: u32,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    pub(super) renderer: RendererKind,
}

#[derive(Debug, Clone, Args)]
pub(super) struct BenchmarkArgs {
    #[arg(value_name = "MIDI")]
    pub(super) midi: PathBuf,
    #[arg(long, default_value_t = 30.0)]
    pub(super) time: f64,
    #[arg(long, default_value_t = 8.0)]
    pub(super) view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    pub(super) time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    pub(super) first_key: u8,
    #[arg(long, default_value_t = 127)]
    pub(super) last_key: u8,
    #[arg(long, default_value_t = 1280)]
    pub(super) width: u32,
    #[arg(long, default_value_t = 720)]
    pub(super) height: u32,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    pub(super) renderer: RendererKind,
    #[arg(long, default_value_t = 120)]
    pub(super) iterations: u32,
    #[arg(long, default_value_t = 10)]
    pub(super) warmup: u32,
}

#[derive(Debug, Clone, Args)]
pub(super) struct RenderVideoArgs {
    #[arg(value_name = "MIDI")]
    pub(super) midi: PathBuf,
    #[arg(long)]
    pub(super) output: PathBuf,
    #[arg(long, default_value_t = 60.0)]
    pub(super) fps: f64,
    #[arg(long, default_value_t = 1920)]
    pub(super) width: u32,
    #[arg(long, default_value_t = 1080)]
    pub(super) height: u32,
    #[arg(long, default_value_t = 8.0)]
    pub(super) view_range: f64,
    #[arg(long, value_enum, default_value_t = DisplayTimeSpace::Time)]
    pub(super) time_space: DisplayTimeSpace,
    #[arg(long, default_value_t = 0)]
    pub(super) first_key: u8,
    #[arg(long, default_value_t = 127)]
    pub(super) last_key: u8,
    #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
    pub(super) renderer: RendererKind,
    #[arg(long, value_enum, default_value_t = FrameColorMode::Premultiplied)]
    pub(super) rgb_mode: FrameColorMode,
    #[arg(long)]
    pub(super) export_alpha_mask: bool,
    #[arg(long, allow_hyphen_values = true)]
    pub(super) ffmpeg_flags: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct RenderAudioArgs {
    #[arg(value_name = "MIDI")]
    pub(super) midi: PathBuf,
    #[arg(long)]
    pub(super) output: PathBuf,
    #[arg(long, default_value_t = 44_100)]
    pub(super) sample_rate: u32,
    #[arg(long, default_value_t = 2)]
    pub(super) channels: u16,
    #[arg(long)]
    pub(super) no_limiter: bool,
    #[arg(long)]
    pub(super) soundfont: Vec<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DebugPianoTrailClassicGeometryArgs {
    #[arg(long, default_value_t = 0)]
    pub(super) first_key: u8,
    #[arg(long, default_value_t = 127)]
    pub(super) last_key: u8,
    #[arg(long, default_value_t = 1280)]
    pub(super) width: u32,
    #[arg(long, default_value_t = 720)]
    pub(super) height: u32,
    #[arg(long, allow_hyphen_values = true)]
    pub(super) scene_json: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProcessCommonArgs {
    #[arg(value_name = "INPUT")]
    pub(super) input: PathBuf,
    #[arg(long)]
    pub(super) output: PathBuf,
    #[arg(long)]
    pub(super) pretty: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProcessSelectArgs {
    #[command(flatten)]
    pub(super) common: ProcessCommonArgs,
    #[arg(long)]
    pub(super) start_ticks: u64,
    #[arg(long)]
    pub(super) end_ticks: u64,
    #[arg(long)]
    pub(super) offset_ticks: Option<u64>,
    #[arg(long)]
    pub(super) track_select: Option<usize>,
    #[arg(long)]
    pub(super) preserve_system_events: bool,
    #[arg(long, value_enum, default_value_t = RangeEdgeBehaviorArg::Trim)]
    pub(super) edge_behavior: RangeEdgeBehaviorArg,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProcessTempoFlattenArgs {
    #[command(flatten)]
    pub(super) common: ProcessCommonArgs,
    #[arg(long)]
    pub(super) tempo: u32,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProcessTempoScaleArgs {
    #[command(flatten)]
    pub(super) common: ProcessCommonArgs,
    #[arg(long)]
    pub(super) factor: f64,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ProcessQuantizeArgs {
    #[command(flatten)]
    pub(super) common: ProcessCommonArgs,
    #[arg(long)]
    pub(super) grid_ticks: u64,
    #[arg(long, value_enum, default_value_t = QuantizeModeArg::NoteStartOnly)]
    pub(super) mode: QuantizeModeArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum AnalysisKindArg {
    File,
    Summary,
    Events,
    Notes,
    Tempo,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum RangeEdgeBehaviorArg {
    Keep,
    Skip,
    Trim,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum QuantizeModeArg {
    NoteStartOnly,
    NoteStartAndEnd,
    AllEvents,
}

pub(super) fn default_analysis_kinds() -> Vec<MidiAnalysisKind> {
    vec![
        MidiAnalysisKind::File,
        MidiAnalysisKind::Summary,
        MidiAnalysisKind::Events,
        MidiAnalysisKind::Notes,
        MidiAnalysisKind::Tempo,
    ]
}

pub(super) fn map_analysis_kind(kind: AnalysisKindArg) -> MidiAnalysisKind {
    match kind {
        AnalysisKindArg::File => MidiAnalysisKind::File,
        AnalysisKindArg::Summary => MidiAnalysisKind::Summary,
        AnalysisKindArg::Events => MidiAnalysisKind::Events,
        AnalysisKindArg::Notes => MidiAnalysisKind::Notes,
        AnalysisKindArg::Tempo => MidiAnalysisKind::Tempo,
    }
}

pub(super) fn map_range_edge_behavior(kind: RangeEdgeBehaviorArg) -> RangeEdgeBehavior {
    match kind {
        RangeEdgeBehaviorArg::Keep => RangeEdgeBehavior::Keep,
        RangeEdgeBehaviorArg::Skip => RangeEdgeBehavior::Skip,
        RangeEdgeBehaviorArg::Trim => RangeEdgeBehavior::Trim,
    }
}

pub(super) fn map_quantize_mode(kind: QuantizeModeArg) -> QuantizeMode {
    match kind {
        QuantizeModeArg::NoteStartOnly => QuantizeMode::NoteStartOnly,
        QuantizeModeArg::NoteStartAndEnd => QuantizeMode::NoteStartAndEnd,
        QuantizeModeArg::AllEvents => QuantizeMode::AllEvents,
    }
}
