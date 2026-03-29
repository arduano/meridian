use clap::{Parser, Subcommand};
use meridian_core::{MeridianError, render::RendererKind};

#[derive(Debug, Parser)]
#[command(name = "meridian-stdio")]
#[command(about = "Stateful JSON-lines frontend for Meridian core")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    ServeJson,
    Json {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        raw: Vec<String>,
    },
    FrameStdout {
        #[arg(value_name = "MIDI")]
        midi: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = meridian_core::protocol::ImageOutputFormat::Rgba)]
        format: meridian_core::protocol::ImageOutputFormat,
        #[arg(long, default_value_t = 0.0)]
        time: f64,
        #[arg(long, default_value_t = 8.0)]
        view_range: f64,
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
    },
    Benchmark {
        #[arg(value_name = "MIDI")]
        midi: std::path::PathBuf,
        #[arg(long, default_value_t = 30.0)]
        time: f64,
        #[arg(long, default_value_t = 8.0)]
        view_range: f64,
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
    },
    RenderVideo {
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
        #[arg(long, default_value_t = 0)]
        first_key: u8,
        #[arg(long, default_value_t = 127)]
        last_key: u8,
        #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
        renderer: RendererKind,
        #[arg(long, allow_hyphen_values = true)]
        ffmpeg_flags: Option<String>,
    },
    DebugMiditrailGeometry {
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
    },
}

pub fn run() -> Result<(), MeridianError> {
    let cli = Cli::parse();
    match cli.command {
        Command::ServeJson => crate::json_mode::serve_json(),
        Command::Json { raw } => crate::json_mode::run_one_json(raw.join(" ").trim()),
        Command::FrameStdout {
            midi,
            format,
            time,
            view_range,
            first_key,
            last_key,
            width,
            height,
            renderer,
        } => crate::frame_stdout::run(
            &midi, format, time, view_range, first_key, last_key, width, height, renderer,
        ),
        Command::Benchmark {
            midi,
            time,
            view_range,
            first_key,
            last_key,
            width,
            height,
            renderer,
            iterations,
            warmup,
        } => crate::benchmark::run(
            &midi, time, view_range, first_key, last_key, width, height, renderer, iterations,
            warmup,
        ),
        Command::RenderVideo {
            midi,
            output,
            fps,
            width,
            height,
            view_range,
            first_key,
            last_key,
            renderer,
            ffmpeg_flags,
        } => crate::render_video::run(
            &midi,
            &output,
            fps,
            width,
            height,
            view_range,
            first_key,
            last_key,
            renderer,
            ffmpeg_flags.as_deref(),
        ),
        Command::DebugMiditrailGeometry {
            first_key,
            last_key,
            width,
            height,
            scene_json,
        } => crate::debug_miditrail::run(first_key, last_key, width, height, scene_json.as_deref()),
    }
}
