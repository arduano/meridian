use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    protocol::{CoreEvent, StateSnapshot},
    render::{
        KeyboardHeightSpec, KeyboardProjectorConfig, NoteProjectorConfig, RendererKind, SceneConfig,
    },
};
use slint::{ModelRc, VecModel};

use super::{inspector::rows_for_scene, view::App};

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
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
    events: &[CoreEvent],
) {
    for event in events {
        apply_event_to_app(app, shared_state, event);
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
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
    update: &UiFrameUpdate,
) {
    apply_state_to_app(app, shared_state, &update.state);
    app.set_visible_note_count_text(update.visible_notes.to_string().into());
    app.set_active_keys_text(update.active_keys.to_string().into());
    app.set_fps_text(update.fps_text.clone().into());
    app.set_status_text(status_text(&update.state).into());
}

fn apply_event_to_app(
    app: &App,
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
    event: &CoreEvent,
) {
    match event {
        CoreEvent::StateSnapshot { state } | CoreEvent::MidiLoaded { state, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_status_text(status_text(state).into());
            app.set_visible_note_count_text("0".into());
            app.set_active_keys_text("0".into());
        }
        CoreEvent::FrameProjected { state, stats, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_visible_note_count_text(stats.visible_notes.to_string().into());
            app.set_active_keys_text(stats.active_keys.to_string().into());
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::FrameSaved { state, output, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_status_text(format!("Saved frame to {}", output.display()).into());
        }
        CoreEvent::VideoRender { .. } | CoreEvent::VideoRenderStatus { .. } => {}
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

fn apply_state_to_app(
    app: &App,
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
    state: &StateSnapshot,
) {
    *shared_state.lock().expect("shared UI state mutex poisoned") = Some(state.clone());
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
        SceneConfig::ThreeD(_) => "3D / Miditrail".into(),
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
