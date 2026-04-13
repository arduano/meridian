//! Active render-export job state and callbacks.
//!
//! `controller.rs` owns the in-flight export draft and terminal state,
//! `callbacks.rs` wires the Slint event handlers, and `jobs.rs` starts the core
//! work. Keeping those responsibilities separate makes export control easier
//! to trace than one monolithic runtime file.

use super::*;

mod callbacks;
mod controller;
mod jobs;

pub(super) use callbacks::wire_render_export_callbacks;
pub(super) use controller::{
    RenderExportController, RenderExportDraft, RenderExportProgress, RenderExportUiSnapshot,
    RenderExportUiTerminal,
};
pub(super) use jobs::start_render_export_jobs;
