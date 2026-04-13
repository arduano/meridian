//! Modify-panel policy registry.
//!
//! The policy table and default tool builders live in `modify_registry/policy.rs`.
//! Per-pass control sync/update handlers live in `modify_pass_controls/passes.rs`.

use super::*;

mod policy;

pub(crate) type ModifyPassPolicy = policy::ModifyPassPolicy;

pub(crate) fn modify_pass_policy(pass_key: &str) -> Option<&'static ModifyPassPolicy> {
    policy::modify_pass_policy(pass_key)
}

pub(crate) fn modify_pass_policy_for_field_prefix(
    field_prefix: &str,
) -> Option<&'static ModifyPassPolicy> {
    policy::modify_pass_policy_for_field_prefix(field_prefix)
}
