use std::io::{self, BufRead, Write};

use clap::{Parser, Subcommand};
use meridian_core::{
    MeridianError,
    protocol::{
        CoreCommand, CoreErrorCode, CoreEvent, ImageOutputFormat, JsonRequest, JsonResponse,
        PROTOCOL_VERSION,
    },
    render::{RendererKind, wgpu::{encode_rgba_to_png, encode_rgba_to_ppm, render_scene_headless_to_rgba}},
    spawn_core,
};

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
