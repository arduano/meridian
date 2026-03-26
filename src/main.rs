mod cli;
mod error;
mod midi;
mod render;
mod ui;

use std::path::Path;

use clap::Parser;
use cli::{AnalyzeCommands, Cli, Commands};
use error::MeridianError;
use midi::{MIDIFileBase, MIDIFileUnion};
use render::{RendererKind, SceneLayout, project_scene};

fn inspect_midi(path: &Path) -> Result<(), MeridianError> {
    let midi = MIDIFileUnion::load_ram(path)?;
    let stats = midi.stats();
    let signature = midi.signature();

    println!("file: {}", signature.filepath.display());
    println!("bytes: {}", signature.length_in_bytes);
    println!("last_modified_us: {}", signature.last_modified);
    println!(
        "length_seconds: {}",
        midi.midi_length()
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "total_notes: {}",
        stats
            .total_notes
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "passed_notes: {}",
        stats
            .passed_notes
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".into())
    );

    Ok(())
}

fn project_frame(
    path: &Path,
    time: f64,
    view_range: f64,
    first_key: u8,
    last_key: u8,
    renderer: RendererKind,
) -> Result<(), MeridianError> {
    let mut midi = MIDIFileUnion::load_ram(path)?;
    let layout = SceneLayout {
        renderer,
        view_range,
        first_key,
        last_key,
        ..Default::default()
    };
    let scene = project_scene(&mut midi, time, &layout);

    println!("file: {}", path.display());
    println!("time_seconds: {time:.3}");
    println!("view_range_seconds: {view_range:.3}");
    println!("visible_notes: {}", scene.visible_notes);
    println!("active_keys: {}", scene.active_keys);
    println!("note_quads: {}", scene.note_quads);
    println!("keyboard_quads: {}", scene.keyboard_quads);
    println!("total_quads: {}", scene.total_quads());
    println!("total_vertices: {}", scene.total_vertices());

    Ok(())
}

fn analyze_summary(path: &Path) -> Result<(), MeridianError> {
    let midi = MIDIFileUnion::load_ram(path)?;
    let stats = midi.stats();
    let summary = midi.analysis_summary();
    let length = midi.midi_length().unwrap_or(0.0);
    let notes_per_second = stats
        .total_notes
        .map(|notes| {
            if length > 0.0 {
                notes as f64 / length
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    println!("file: {}", path.display());
    println!("length_seconds: {:.3}", length);
    println!("total_notes: {}", stats.total_notes.unwrap_or(0));
    println!("notes_per_second: {:.3}", notes_per_second);
    println!("keys_with_notes: {}", summary.keys_with_notes);
    println!("total_blocks: {}", summary.total_blocks);
    println!("max_blocks_per_key: {}", summary.max_blocks_per_key);
    println!("max_notes_in_block: {}", summary.max_notes_in_block);
    println!("densest_key: {}", summary.densest_key);
    println!("densest_key_notes: {}", summary.densest_key_notes);

    Ok(())
}

fn analyze_keys(path: &Path, top: usize) -> Result<(), MeridianError> {
    let midi = MIDIFileUnion::load_ram(path)?;
    let mut keyed: Vec<_> = midi
        .key_note_counts()
        .into_iter()
        .enumerate()
        .filter(|(_, notes)| *notes > 0)
        .collect();
    keyed.sort_by_key(|(_, notes)| std::cmp::Reverse(*notes));

    println!("file: {}", path.display());
    println!("top_keys: {}", top);
    for (rank, (key, notes)) in keyed.into_iter().take(top).enumerate() {
        println!("rank_{}: key={} notes={}", rank + 1, key, notes);
    }

    Ok(())
}

fn analyze_density(
    path: &Path,
    samples: usize,
    view_range: f64,
    first_key: u8,
    last_key: u8,
) -> Result<(), MeridianError> {
    let mut midi = MIDIFileUnion::load_ram(path)?;
    let length = midi.midi_length().unwrap_or(0.0);
    let sample_count = samples.max(1);
    let first_key = first_key.min(last_key) as usize;
    let last_key = last_key.max(first_key as u8) as usize;

    let mut peak_visible_notes = 0_usize;
    let mut peak_visible_notes_time = 0.0;
    let mut peak_active_keys = 0_usize;
    let mut peak_active_keys_time = 0.0;
    let mut visible_sum = 0_u64;
    let mut active_sum = 0_u64;

    for sample_index in 0..sample_count {
        let time = if sample_count == 1 || length <= 0.0 {
            0.0
        } else {
            (sample_index as f64 / (sample_count - 1) as f64) * length
        };

        let views = midi.get_current_column_views(time, view_range);
        let mut visible_notes = 0_usize;
        let mut active_keys = 0_usize;

        for key in first_key..=last_key.min(midi::MIDI_KEY_COUNT - 1) {
            let column = views.get_column(key);
            let notes: Vec<_> = column.iterate_displaced_notes().collect();
            let visible_for_key = notes
                .iter()
                .filter(|note| {
                    let end = note.start + note.len;
                    end > 0.0 && note.start < view_range as f32
                })
                .count();
            let active_for_key = notes
                .iter()
                .any(|note| note.start <= 0.0 && note.start + note.len > 0.0);

            visible_notes += visible_for_key;
            if active_for_key {
                active_keys += 1;
            }
        }

        visible_sum += visible_notes as u64;
        active_sum += active_keys as u64;

        if visible_notes > peak_visible_notes {
            peak_visible_notes = visible_notes;
            peak_visible_notes_time = time;
        }
        if active_keys > peak_active_keys {
            peak_active_keys = active_keys;
            peak_active_keys_time = time;
        }
    }

    println!("file: {}", path.display());
    println!("samples: {}", sample_count);
    println!("view_range_seconds: {:.3}", view_range);
    println!(
        "key_range: {}..={}",
        first_key,
        last_key.min(midi::MIDI_KEY_COUNT - 1)
    );
    println!(
        "average_visible_notes: {:.3}",
        visible_sum as f64 / sample_count as f64
    );
    println!("peak_visible_notes: {}", peak_visible_notes);
    println!("peak_visible_notes_time: {:.3}", peak_visible_notes_time);
    println!(
        "average_active_keys: {:.3}",
        active_sum as f64 / sample_count as f64
    );
    println!("peak_active_keys: {}", peak_active_keys);
    println!("peak_active_keys_time: {:.3}", peak_active_keys_time);

    Ok(())
}

fn main() -> Result<(), MeridianError> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Inspect { midi }) => inspect_midi(&midi),
        Some(Commands::ProjectFrame {
            midi,
            time,
            view_range,
            first_key,
            last_key,
            renderer,
        }) => project_frame(&midi, time, view_range, first_key, last_key, renderer),
        Some(Commands::Analyze { command }) => match command {
            AnalyzeCommands::Summary { midi } => analyze_summary(&midi),
            AnalyzeCommands::Density {
                midi,
                samples,
                view_range,
                first_key,
                last_key,
            } => analyze_density(&midi, samples, view_range, first_key, last_key),
            AnalyzeCommands::Keys { midi, top } => analyze_keys(&midi, top),
        },
        Some(Commands::Ui {
            midi,
            time,
            view_range,
            first_key,
            last_key,
            renderer,
            disable_wgpu,
        }) => ui::run_ui(ui::UiOptions {
            midi_path: midi,
            start_time: time,
            view_range,
            first_key,
            last_key,
            renderer,
            disable_wgpu,
        }),
        None => ui::run_ui(ui::UiOptions::default()),
    }
}
