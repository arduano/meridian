use std::{cell::RefCell, env, path::PathBuf, rc::Rc};

use meridian_core::{
    RenderedFrame,
    protocol::{CoreEvent, StateSnapshot},
    render::RendererKind,
};

use super::view::App;

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
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    events: &[CoreEvent],
) {
    for event in events {
        apply_event_to_app(app, shared_state, event);
    }
}

pub fn apply_rendered_frame_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    frame: &RenderedFrame,
) {
    apply_state_to_app(app, shared_state, &frame.state);
    app.set_visible_note_count_text(frame.stats.visible_notes.to_string().into());
    app.set_active_keys_text(frame.stats.active_keys.to_string().into());
    app.set_status_text(status_text(&frame.state).into());
}

fn apply_event_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
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
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

fn apply_state_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    state: &StateSnapshot,
) {
    *shared_state.borrow_mut() = Some(state.clone());
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
