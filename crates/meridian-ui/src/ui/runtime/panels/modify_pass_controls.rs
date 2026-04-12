//! Per-pass modify control synchronization.
//!
//! This module bridges the modify registry's policy table and the generated
//! Slint controls for each pass.

use super::*;

mod sync;
pub(super) use sync::sync_modify_pass_controls;
mod update;
pub(super) use update::update_modify_control;
