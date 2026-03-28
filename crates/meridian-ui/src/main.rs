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
    #[arg(long, default_value_t = 0.0)]
    time: f64,
    #[arg(long, default_value_t = 8.0)]
    view_range: f64,
    #[arg(long, default_value_t = 0)]
    first_key: u8,
    #[arg(long, default_value_t = 127)]
    last_key: u8,
    #[arg(long, value_parser = parse_renderer, default_value = "pfa")]
    renderer: RendererKind,
    #[arg(long, env = "MERIDIAN_DISABLE_WGPU", default_value_t = false)]
    disable_wgpu: bool,
}

fn parse_renderer(value: &str) -> Result<RendererKind, String> {
    match value {
        "flat" => Ok(RendererKind::Flat),
        "pfa" => Ok(RendererKind::Pfa),
        _ => Err(format!("unknown renderer `{value}`")),
    }
}

fn main() -> Result<(), meridian_core::MeridianError> {
    let cli = Cli::parse();
    ui::run_ui(ui::UiOptions {
        midi_path: cli.midi,
        renderer: cli.renderer,
        start_time: cli.time,
        view_range: cli.view_range,
        first_key: cli.first_key,
        last_key: cli.last_key,
        disable_wgpu: cli.disable_wgpu,
    })
}
