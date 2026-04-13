//! Modify-panel policy registry.
//!
//! The policy table and default tool builders live in `modify_registry/policy.rs`.
//! Per-pass control sync/update handlers live in `modify_pass_controls/passes.rs`.

use super::*;

mod policy;

#[allow(unused_imports)]
pub(crate) use policy::{
    modify_pass_policy, modify_pass_policy_for_field_prefix, ModifyPassPolicy,
};
