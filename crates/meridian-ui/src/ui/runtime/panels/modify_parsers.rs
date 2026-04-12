//! Modify-panel text parsing helpers.
//!
//! This module is the shared parsing layer for modify controls. The
//! `collections`, `core`, and `timing` submodules each focus on a narrow shape
//! of input so the pass handlers can stay small.

use super::*;

mod collections;
pub(super) use collections::*;
mod core;
pub(super) use core::*;
mod timing;
pub(super) use timing::*;
