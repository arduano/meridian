//! Per-pass modify control synchronization.
//!
//! This module bridges the modify registry's policy table and the generated
//! Slint controls for each pass.

use super::*;

mod sync;
pub(crate) use sync::sync_modify_pass_controls;
mod update;
pub(crate) use update::update_modify_control;
mod passes;
pub(crate) use passes::*;
