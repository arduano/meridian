use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    protocol::{CoreEvent, MidiAnalysisData, StateSnapshot},
    render::{
        KeyboardHeightSpec, KeyboardProjectorConfig, NoteProjectorConfig, RendererKind,
        SceneConfig, ThreeDSceneConfig,
    },
};
use slint::{ModelRc, VecModel};

use super::{inspector::rows_for_scene, view::App, view_model::UiViewModel};

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub midi_path: Option<PathBuf>,
    pub renderer: RendererKind,
    pub start_time: f64,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    pub disable_wgpu: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            midi_path: None,
            renderer: RendererKind::Pfa,
            start_time: 0.0,
            view_range: 8.0,
            first_key: 0,
            last_key: 127,
            disable_wgpu: matches!(
                env::var("MERIDIAN_DISABLE_WGPU").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

pub fn apply_events_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    events: &[CoreEvent],
) {
    let snapshot = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .clone();
    if let Some(state) = snapshot.as_ref() {
        apply_state_to_app(app, shared_state, state);
        for event in events {
            apply_event_overrides_to_app(app, event, state);
        }
    } else {
        for event in events {
            if let CoreEvent::Error { message, .. } = event {
                app.set_status_text(message.clone().into());
            }
        }
    }
}

#[derive(Clone)]
pub struct UiFrameUpdate {
    pub state: StateSnapshot,
    pub visible_notes: usize,
    pub active_keys: usize,
    pub fps_text: String,
}

pub fn apply_frame_update_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    update: &UiFrameUpdate,
) {
    apply_state_to_app(app, shared_state, &update.state);
    app.set_visible_note_count_text(update.visible_notes.to_string().into());
    app.set_active_keys_text(update.active_keys.to_string().into());
    app.set_fps_text(update.fps_text.clone().into());
    app.set_status_text(status_text(&update.state).into());
}

fn apply_event_overrides_to_app(app: &App, event: &CoreEvent, state: &StateSnapshot) {
    match event {
        CoreEvent::StateSnapshot { .. }
        | CoreEvent::MidiLoaded { .. }
        | CoreEvent::ProcessedMidiAttached { .. }
        | CoreEvent::DisplayCacheAttached { .. }
        | CoreEvent::AudioCacheAttached { .. } => {
            app.set_status_text(status_text(state).into());
            app.set_visible_note_count_text("0".into());
            app.set_active_keys_text("0".into());
        }
        CoreEvent::ProcessedMidiBuilt {
            midi_length,
            total_notes,
            track_count,
            ..
        } => {
            app.set_analysis_note_count_text(total_notes.to_string().into());
            app.set_analysis_track_count_text(track_count.to_string().into());
            app.set_analysis_midi_length_text(format!("{:.3} s", midi_length).into());
        }
        CoreEvent::MidiAnalysis { analysis, .. } => {
            apply_analysis_to_app(app, analysis);
        }
        CoreEvent::FrameProjected { stats, .. } => {
            app.set_visible_note_count_text(stats.visible_notes.to_string().into());
            app.set_active_keys_text(stats.active_keys.to_string().into());
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::FrameSaved { output, .. } => {
            app.set_status_text(format!("Saved frame to {}", output.display()).into());
        }
        CoreEvent::VideoRender { .. }
        | CoreEvent::VideoRenderStatus { .. }
        | CoreEvent::AudioRender { .. }
        | CoreEvent::AudioRenderStatus { .. }
        | CoreEvent::AudioStatus { .. }
        | CoreEvent::MidiLoadProgress { .. }
        | CoreEvent::ParsedMidiLoaded { .. }
        | CoreEvent::DisplayCacheBuilt { .. }
        | CoreEvent::AudioCacheBuilt { .. }
        | CoreEvent::DisplaySessionCreated { .. }
        | CoreEvent::AudioSessionCreated { .. } => {}
        CoreEvent::DisplaySessionAttached { .. } | CoreEvent::AudioSessionAttached { .. } => {
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

fn apply_state_to_app(app: &App, shared_state: &Arc<Mutex<UiViewModel>>, state: &StateSnapshot) {
    shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .apply_snapshot(state);
    app.set_midi_path_text(
        state
            .midi_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No MIDI loaded".into())
            .into(),
    );
    app.set_time_text(format!("{:.3} s", state.current_time).into());
    app.set_length_text(format!("{:.3} s", state.midi_length).into());
    app.set_note_count_text(state.total_notes.to_string().into());
    app.set_view_range_text(format!("{:.1} s", state.view_range).into());
    app.set_current_time_seconds(state.current_time as f32);
    app.set_midi_length_seconds(state.midi_length.max(0.001) as f32);
    app.set_scene_summary_text(scene_summary(&state.scene).into());
    app.set_current_renderer_text(renderer_summary(&state.scene).into());
    app.set_time_space_text(match state.time_space {
        meridian_core::render::DisplayTimeSpace::Time => "time".into(),
        meridian_core::render::DisplayTimeSpace::Tick => "tick".into(),
    });
    app.set_viewport_text(format!("{} x {}", state.viewport_width, state.viewport_height).into());
    app.set_inspector_items(ModelRc::from(std::rc::Rc::new(VecModel::from(
        rows_for_scene(&state.scene),
    ))));
    app.set_play_label(if state.playing {
        "Pause".into()
    } else {
        "Play".into()
    });
}

fn status_text(state: &StateSnapshot) -> String {
    if let Some(path) = &state.midi_path {
        format!(
            "Core session active. UI frontend attached. Rendering {} through the shared event bus.",
            path.display()
        )
    } else {
        "Launch with `meridian-ui --midi <file.mid>` to render a MIDI".into()
    }
}

fn scene_summary(scene: &SceneConfig) -> String {
    match scene {
        SceneConfig::TwoD(scene) => format!(
            "2D / {} notes / {} keyboard / {}",
            note_name(&scene.notes),
            keyboard_name(&scene.keyboard),
            height_name(&scene.keyboard_height),
        ),
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => {
            "3D / Piano Trail Classic".into()
        }
    }
}

fn note_name(config: &NoteProjectorConfig) -> &'static str {
    match config {
        NoteProjectorConfig::Flat(_) => "Flat",
        NoteProjectorConfig::Pfa(_) => "PFA",
    }
}

fn keyboard_name(config: &KeyboardProjectorConfig) -> &'static str {
    match config {
        KeyboardProjectorConfig::Flat(_) => "Flat",
        KeyboardProjectorConfig::Pfa(_) => "PFA",
    }
}

fn height_name(config: &KeyboardHeightSpec) -> &'static str {
    match config {
        KeyboardHeightSpec::ScreenPercent { .. } => "screen %",
        KeyboardHeightSpec::AspectRatio { .. } => "aspect",
    }
}

fn renderer_summary(scene: &SceneConfig) -> &'static str {
    match scene {
        SceneConfig::TwoD(scene) => match (&scene.notes, &scene.keyboard) {
            (NoteProjectorConfig::Pfa(_), KeyboardProjectorConfig::Pfa(_)) => "pfa",
            (NoteProjectorConfig::Flat(_), KeyboardProjectorConfig::Flat(_)) => "flat",
            _ => "mixed",
        },
        SceneConfig::ThreeD(_) => "3d",
    }
}

fn apply_analysis_to_app(app: &App, analysis: &MidiAnalysisData) {
    app.set_analysis_note_count_text(analysis.total_notes.to_string().into());
    app.set_analysis_midi_length_text(format!("{:.3} s", analysis.midi_length).into());

    let first_key = analysis
        .key_note_counts
        .iter()
        .position(|count| *count > 0)
        .unwrap_or(0);
    let last_key = analysis
        .key_note_counts
        .iter()
        .rposition(|count| *count > 0)
        .unwrap_or(0);
    app.set_analysis_key_range_text(format!("{first_key}..{last_key}").into());

    let bucket_width = if analysis.buckets.len() > 1 {
        analysis.buckets[1].time_seconds - analysis.buckets[0].time_seconds
    } else {
        analysis.midi_length.max(0.5)
    }
    .max(0.001);
    let peak_nps = analysis
        .buckets
        .iter()
        .map(|bucket| bucket.note_starts as f64 / bucket_width)
        .fold(0.0, f64::max);
    let avg_nps = if analysis.midi_length > 0.0 {
        analysis.total_notes as f64 / analysis.midi_length
    } else {
        analysis.total_notes as f64
    };
    app.set_analysis_note_density_text(
        format!("avg {:.1}/s peak {:.1}/s", avg_nps, peak_nps).into(),
    );

    app.set_analysis_tempo_text("From bucketed analysis".into());
    app.set_analysis_time_signature_text("Not computed".into());
    app.set_analysis_avg_velocity_text("Not computed".into());
}
