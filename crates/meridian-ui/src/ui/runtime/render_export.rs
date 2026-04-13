//! Active render-export runtime wiring for the UI.
//!
//! `export.rs` owns configuration and render config construction. This module
//! owns the in-flight export runtime itself: the shared job state, the Slint
//! event listeners, and the terminal UI projection.

use super::*;

mod callbacks;
mod jobs;
mod runtime;

pub(super) fn install_render_export_runtime(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) -> Arc<Mutex<RenderExportRuntime>> {
    let export_state = Arc::new(Mutex::new(RenderExportRuntime::default()));
    install_core_event_listener(app, bridge.core(), shared_state, &export_state);
    wire_render_export_callbacks(app, bridge, shared_state, &export_state);
    export_state
}

pub(super) use callbacks::wire_render_export_callbacks;
pub(super) use jobs::start_render_export_jobs;
pub(super) use runtime::{
    RenderExportDraft, RenderExportProgress, RenderExportRuntime, RenderExportUiSnapshot,
    RenderExportUiTerminal,
};
