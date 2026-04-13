use std::time::Instant;

use meridian_core::{
    MeridianError,
    midi::backend::MIDIFileUnion,
    render::{
        DisplayTimeSpace, RendererKind, SceneLayout, headless::HeadlessRenderSession, project_scene,
    },
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct BenchmarkSummary {
    midi: std::path::PathBuf,
    renderer: RendererKind,
    time: f64,
    view_range: f64,
    time_space: DisplayTimeSpace,
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

#[expect(
    clippy::too_many_arguments,
    reason = "CLI entrypoints mirror parsed flags directly before they are folded into the runtime layout."
)]
pub fn run(
    midi: &std::path::Path,
    time: f64,
    view_range: f64,
    time_space: DisplayTimeSpace,
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

    let mut layout = SceneLayout {
        scene: Default::default(),
        view_range,
        time_space,
        first_key,
        last_key,
        viewport_width: width,
        viewport_height: height,
    };
    layout.set_renderer_kind(renderer);

    let midi_path = midi.to_path_buf();
    let mut midi = MIDIFileUnion::load_ram(midi)?;
    let mut session = HeadlessRenderSession::new(&layout, width, height)?;

    for _ in 0..warmup {
        let scene = project_scene(&mut midi, time, None, &layout);
        session.render(&layout, &scene);
    }

    let mut projection_ms = Vec::with_capacity(iterations as usize);
    let mut gpu_render_ms = Vec::with_capacity(iterations as usize);
    let mut total_ms = Vec::with_capacity(iterations as usize);
    let mut frame_stats = None;

    for _ in 0..iterations {
        let total_start = Instant::now();

        let projection_start = Instant::now();
        let scene = project_scene(&mut midi, time, None, &layout);
        let projection_elapsed = projection_start.elapsed().as_secs_f64() * 1_000.0;

        let gpu_start = Instant::now();
        session.render(&layout, &scene);
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
        time_space,
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
