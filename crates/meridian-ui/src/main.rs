mod ui;

use std::path::PathBuf;

use clap::Parser;
use meridian_core::render::RendererKind;

#[derive(Debug, Parser)]
#[command(name = "meridian-ui")]
#[command(about = "Slint frontend for the Meridian core")]
struct Cli {
    #[arg(long)]
    midi: Option<PathBuf>,
    #[arg(long)]
    time: Option<f64>,
    #[arg(long)]
    view_range: Option<f64>,
    #[arg(long)]
    first_key: Option<u8>,
    #[arg(long)]
    last_key: Option<u8>,
    #[arg(long, value_parser = parse_renderer)]
    renderer: Option<RendererKind>,
    #[arg(long, env = "MERIDIAN_DISABLE_WGPU", default_value_t = false)]
    disable_wgpu: bool,
    #[cfg(feature = "debug-snapshots")]
    #[arg(long, hide = true)]
    debug_snapshot_output: Option<PathBuf>,
    #[cfg(feature = "debug-snapshots")]
    #[arg(long, hide = true, default_value_t = 1540)]
    debug_snapshot_width: u32,
    #[cfg(feature = "debug-snapshots")]
    #[arg(long, hide = true, default_value_t = 940)]
    debug_snapshot_height: u32,
}

fn parse_renderer(value: &str) -> Result<RendererKind, String> {
    match value {
        "flat" => Ok(RendererKind::Flat),
        "pfa" => Ok(RendererKind::Pfa),
        "piano_trail_classic" | "piano-trail-classic" => Ok(RendererKind::PianoTrailClassic),
        _ => Err(format!("unknown renderer `{value}`")),
    }
}

fn main() -> Result<(), meridian_core::MeridianError> {
    let cli = Cli::parse();
    let options = ui::UiOptions {
        midi_path: cli.midi,
        renderer: cli.renderer,
        start_time: cli.time,
        view_range: cli.view_range,
        first_key: cli.first_key,
        last_key: cli.last_key,
        disable_wgpu: cli.disable_wgpu,
    };
    #[cfg(feature = "debug-snapshots")]
    if let Some(output) = cli.debug_snapshot_output {
        return ui::write_debug_snapshot(
            options,
            &output,
            cli.debug_snapshot_width,
            cli.debug_snapshot_height,
        );
    }
    ui::run_ui(options)
}
