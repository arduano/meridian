use super::*;

pub(crate) fn update_modify_control(app: &App, key: &str, value: &str) -> Result<(), String> {
    let mut config = fallback_modify_config(app);
    let (field_prefix, field_key) = key
        .split_once('.')
        .ok_or_else(|| format!("unknown modify control: {key}"))?;
    let policy = modify_pass_policy_for_field_prefix(field_prefix)
        .ok_or_else(|| format!("unknown modify control: {key}"))?;

    (policy.update_control)(app, &mut config, field_key, value)?;
    set_modify_config(app, &config);
    Ok(())
}
