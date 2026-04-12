//! Modify-panel initialization and wiring entrypoint.
//!
//! `modify_registry.rs` owns the pass table, `modify_controls.rs` owns the
//! shared validation helpers, and `modify_pass_controls.rs` handles the
//! per-pass Slint bindings.

use super::*;

mod wiring;
use wiring::*;

pub(in super::super) fn initialize_modify_panel(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    load_modify_pass_into_app(app, "quantize");
    if app.get_modify_output_path_text().is_empty() && !app.get_selected_midi_name().is_empty() {
        app.set_modify_output_path_text(
            default_modify_output_path(app.get_selected_midi_name().as_str())
                .display()
                .to_string()
                .into(),
        );
    }
    if let Ok(status) = bridge.get_midi_file_process_status(shared_state) {
        apply_events_to_app(
            app,
            shared_state,
            &[CoreEvent::MidiProcessStatus { status }],
        );
    }
}

pub(in super::super) fn wire_modify_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    wire_modify_pass_selection(app);
    wire_modify_control_updates(app);
    wire_modify_config_editor(app);
    wire_modify_config_reset(app);
    wire_modify_output_browse(app);
    wire_modify_job_actions(app, bridge, shared_state);
}
