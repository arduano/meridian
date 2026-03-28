use std::{
    io::{self, BufRead, Write},
    time::Instant,
};

use clap::{Parser, Subcommand};
use meridian_core::{
    MeridianError,
    midi::backend::MIDIFileUnion,
    protocol::{
        CoreCommand, CoreErrorCode, CoreEvent, ImageOutputFormat, JsonRequest, JsonResponse,
        PROTOCOL_VERSION,
    },
    render::{
        RendererKind, SceneLayout,
        pfa::wgpu::{
            HeadlessRenderSession, encode_rgba_to_png, encode_rgba_to_ppm,
            render_scene_headless_to_rgba,
        },
        project_scene,
    },
    spawn_core,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "meridian-stdio")]
#[command(about = "Stateful JSON-lines frontend for Meridian core")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a persistent JSON-lines control loop over stdin/stdout
    ServeJson,
    /// Execute one raw JSON request and print a JSON response
    Json {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        raw: Vec<String>,
    },
    /// Load a MIDI into a fresh core session and write one rendered frame to stdout
    FrameStdout {
        #[arg(value_name = "MIDI")]
        midi: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = ImageOutputFormat::Rgba)]
        format: ImageOutputFormat,
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
    /// Benchmark repeated projection and headless GPU rendering at a fixed MIDI timestamp
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
}

fn main() -> Result<(), MeridianError> {
    let cli = Cli::parse();
    match cli.command {
        Command::ServeJson => serve_json(),
        Command::Json { raw } => run_one_json(raw.join(" ").trim()),
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
        } => frame_stdout(
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
        } => benchmark(
            &midi, time, view_range, first_key, last_key, width, height, renderer, iterations,
            warmup,
        ),
    }
}

fn serve_json() -> Result<(), MeridianError> {
    let core = spawn_core();
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<JsonRequest>(trimmed) {
            Ok(request) => request,
            Err(error) => {
                write_json(
                    &mut stdout,
                    &JsonResponse {
                        protocol_version: PROTOCOL_VERSION,
                        id: None,
                        events: vec![CoreEvent::Error {
                            code: CoreErrorCode::InvalidJson,
                            message: format!("invalid_json: {error}"),
                        }],
                    },
                )?;
                continue;
            }
        };

        if request.protocol_version != PROTOCOL_VERSION {
            write_json(
                &mut stdout,
                &JsonResponse {
                    protocol_version: PROTOCOL_VERSION,
                    id: request.id,
                    events: vec![CoreEvent::Error {
                        code: CoreErrorCode::InvalidProtocolVersion,
                        message: format!(
                            "unsupported protocol_version {}, expected {}",
                            request.protocol_version, PROTOCOL_VERSION
                        ),
                    }],
                },
            )?;
            continue;
        }

        let should_shutdown = matches!(request.command, CoreCommand::Shutdown);
        let response = JsonResponse {
            protocol_version: PROTOCOL_VERSION,
            id: request.id,
            events: core.request(request.command)?,
        };
        write_json(&mut stdout, &response)?;

        if should_shutdown {
            break;
        }
    }

    Ok(())
}

fn run_one_json(raw: &str) -> Result<(), MeridianError> {
    let request: JsonRequest = serde_json::from_str(raw)
        .map_err(|error| MeridianError::InvalidMidi(format!("invalid json request: {error}")))?;
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(MeridianError::InvalidMidi(format!(
            "unsupported protocol_version {}, expected {}",
            request.protocol_version, PROTOCOL_VERSION
        )));
    }
    let core = spawn_core();
    let response = JsonResponse {
        protocol_version: PROTOCOL_VERSION,
        id: request.id,
        events: core.request(request.command)?,
    };
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?
    );
    if !matches!(
        response.events.as_slice(),
        [meridian_core::protocol::CoreEvent::ShutdownComplete]
    ) {
        let _ = core.request(CoreCommand::Shutdown);
    }
    Ok(())
}

fn write_json(stdout: &mut impl Write, response: &JsonResponse) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, response)
        .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn frame_stdout(
    midi: &std::path::Path,
    format: ImageOutputFormat,
    time: f64,
    view_range: f64,
    first_key: u8,
    last_key: u8,
    width: u32,
    height: u32,
    renderer: RendererKind,
) -> Result<(), MeridianError> {
    let core = spawn_core();
    core.request(CoreCommand::SetLayout {
        renderer: Some(renderer),
        view_range: Some(view_range),
        first_key: Some(first_key),
        last_key: Some(last_key),
        viewport_width: Some(width),
        viewport_height: Some(height),
    })?;
    core.request(CoreCommand::LoadMidi {
        path: midi.to_path_buf(),
    })?;
    core.request(CoreCommand::SetTime { time })?;

    let frame = core.render_frame(Some(width), Some(height))?;
    let rgba = render_scene_headless_to_rgba(width, height, &frame.scene)?;
    let bytes = match format {
        ImageOutputFormat::Rgba => rgba,
        ImageOutputFormat::Ppm => encode_rgba_to_ppm(width, height, &rgba),
        ImageOutputFormat::Png => encode_rgba_to_png(width, height, &rgba)?,
    };

    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.flush()?;
    let _ = core.request(CoreCommand::Shutdown);
    Ok(())
}

#[derive(Debug, Serialize)]
struct BenchmarkSummary {
    midi: std::path::PathBuf,
    renderer: RendererKind,
    time: f64,
    view_range: f64,
    width: u32,
    height: u32,
    iterations: u32,
    warmup: u32,
    frame_stats: meridian_core::protocol::FrameStats,
    projection_ms: TimingSummary,
    gpu_render_ms: TimingSummary,
    total_ms: TimingSummary,
    projected_quads_per_second: f64,
    rendered_quads_per_second: f64,
}

#[derive(Debug, Serialize)]
struct TimingSummary {
    avg: f64,
    min: f64,
    max: f64,
}

fn benchmark(
    midi: &std::path::Path,
    time: f64,
    view_range: f64,
    first_key: u8,
    last_key: u8,
    width: u32,
    height: u32,
    renderer: RendererKind,
    iterations: u32,
    warmup: u32,
) -> Result<(), MeridianError> {
    if iterations == 0 {
        return Err(MeridianError::InvalidMidi(
            "iterations must be greater than zero".into(),
        ));
    }

    let layout = SceneLayout {
        renderer,
        view_range,
        piano_height: SceneLayout::default().piano_height,
        first_key,
        last_key,
        viewport_width: width,
        viewport_height: height,
    };
    let midi_path = midi.to_path_buf();
    let mut midi = MIDIFileUnion::load_ram(midi)?;
    let mut session = HeadlessRenderSession::new(width, height)?;
    for _ in 0..warmup {
        let scene = project_scene(&mut midi, time, &layout);
        session.render_blocking(&scene)?;
    }

    let mut projection_ms = Vec::with_capacity(iterations as usize);
    let mut gpu_render_ms = Vec::with_capacity(iterations as usize);
    let mut total_ms = Vec::with_capacity(iterations as usize);
    let mut frame_stats = None;

    for _ in 0..iterations {
        let total_start = Instant::now();

        let projection_start = Instant::now();
        let scene = project_scene(&mut midi, time, &layout);
        let projection_elapsed = projection_start.elapsed().as_secs_f64() * 1_000.0;

        let gpu_start = Instant::now();
        session.render_blocking(&scene)?;
        let gpu_elapsed = gpu_start.elapsed().as_secs_f64() * 1_000.0;

        projection_ms.push(projection_elapsed);
        gpu_render_ms.push(gpu_elapsed);
        total_ms.push(total_start.elapsed().as_secs_f64() * 1_000.0);
        frame_stats = Some(meridian_core::protocol::FrameStats {
            visible_notes: scene.visible_notes,
            active_keys: scene.active_keys,
            note_quads: scene.note_quads,
            keyboard_quads: scene.keyboard_quads,
            total_quads: scene.total_quads(),
            total_vertices: scene.total_vertices(),
        });
    }

    let frame_stats = frame_stats.expect("benchmark iterations > 0");
    let summary = BenchmarkSummary {
        midi: midi_path,
        renderer,
        time,
        view_range,
        width,
        height,
        iterations,
        warmup,
        projected_quads_per_second: frame_stats.total_quads as f64
            / (timing_summary(&projection_ms).avg / 1_000.0).max(f64::EPSILON),
        rendered_quads_per_second: frame_stats.total_quads as f64
            / (timing_summary(&gpu_render_ms).avg / 1_000.0).max(f64::EPSILON),
        frame_stats,
        projection_ms: timing_summary(&projection_ms),
        gpu_render_ms: timing_summary(&gpu_render_ms),
        total_ms: timing_summary(&total_ms),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&summary)
            .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?
    );
    Ok(())
}

fn timing_summary(samples: &[f64]) -> TimingSummary {
    let mut min = f64::INFINITY;
    let mut max = 0.0_f64;
    let mut sum = 0.0_f64;
    for &sample in samples {
        min = min.min(sample);
        max = max.max(sample);
        sum += sample;
    }
    TimingSummary {
        avg: sum / samples.len() as f64,
        min,
        max,
    }
}
