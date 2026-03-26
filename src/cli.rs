use std::path::PathBuf;

use crate::render::RendererKind;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "meridian")]
#[command(about = "Primitive Zenith-style MIDI renderer prototype")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch the Slint + WGPU UI
    Ui {
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
        #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
        renderer: RendererKind,
        #[arg(long, env = "MERIDIAN_DISABLE_WGPU", default_value_t = false)]
        disable_wgpu: bool,
    },

    /// Load a MIDI with the RAM backend and print basic stats
    Inspect {
        #[arg(value_name = "MIDI")]
        midi: PathBuf,
    },

    /// Load a MIDI, project one frame, and print render-facing stats
    ProjectFrame {
        #[arg(value_name = "MIDI")]
        midi: PathBuf,
        #[arg(long, default_value_t = 0.0)]
        time: f64,
        #[arg(long, default_value_t = 8.0)]
        view_range: f64,
        #[arg(long, default_value_t = 0)]
        first_key: u8,
        #[arg(long, default_value_t = 127)]
        last_key: u8,
        #[arg(long, value_enum, default_value_t = RendererKind::Pfa)]
        renderer: RendererKind,
    },

    /// MIDI analysis commands for loader and view-window validation
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum AnalyzeCommands {
    /// Print structural stats from the RAM MIDI backend
    Summary {
        #[arg(value_name = "MIDI")]
        midi: PathBuf,
    },

    /// Sample view-density over time using the shared note-view abstractions
    Density {
        #[arg(value_name = "MIDI")]
        midi: PathBuf,
        #[arg(long, default_value_t = 64)]
        samples: usize,
        #[arg(long, default_value_t = 8.0)]
        view_range: f64,
        #[arg(long, default_value_t = 0)]
        first_key: u8,
        #[arg(long, default_value_t = 127)]
        last_key: u8,
    },

    /// Print the busiest keys by total note count
    Keys {
        #[arg(value_name = "MIDI")]
        midi: PathBuf,
        #[arg(long, default_value_t = 16)]
        top: usize,
    },
}
