use clap::{Parser, Subcommand};
use meridian_core::{
    MeridianError,
    render::{DisplayTimeSpace, RendererKind},
};

#[derive(Debug, Parser)]
#[command(name = "meridian")]
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
        Command::Render {
            command: RenderCommand::Frame(args),
        }
        | Command::FrameStdout(args) => {
            run_frame_stdout(args)
        }
        Command::Render {
            command: RenderCommand::Video(args),
        }
        | Command::RenderVideo(args) => {
            run_render_video(args)
        }
        Command::Render {
            command: RenderCommand::Audio(args),
        }
        | Command::RenderAudio(args) => {
            run_render_audio(args)
        }
        Command::Bench(args) | Command::Benchmark(args) => run_benchmark(args),
        Command::Debug {
            command: DebugCommand::PianoTrailClassicGeometry(args),
        }
        | Command::DebugPianoTrailClassicGeometry(args) => run_debug_geometry(args),
    }
}

#[derive(Debug, clap::Args)]
pub struct JsonArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    raw: Vec<String>,
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

#[derive(Debug, clap::Args)]
pub struct FrameStdoutArgs {
    #[arg(value_name = "MIDI")]
    midi: std::path::PathBuf,
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

#[derive(Debug, clap::Args)]
pub struct BenchmarkArgs {
    #[arg(value_name = "MIDI")]
    midi: std::path::PathBuf,
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

#[derive(Debug, clap::Args)]
pub struct RenderVideoArgs {
    #[arg(value_name = "MIDI")]
    midi: std::path::PathBuf,
    #[arg(long)]
    output: std::path::PathBuf,
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

#[derive(Debug, clap::Args)]
pub struct RenderAudioArgs {
    #[arg(value_name = "MIDI")]
    midi: std::path::PathBuf,
    #[arg(long)]
    output: std::path::PathBuf,
    #[arg(long, default_value_t = 44_100)]
    sample_rate: u32,
    #[arg(long, default_value_t = 2)]
    channels: u16,
    #[arg(long)]
    no_limiter: bool,
    #[arg(long)]
    soundfont: Vec<std::path::PathBuf>,
}

#[derive(Debug, clap::Args)]
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
