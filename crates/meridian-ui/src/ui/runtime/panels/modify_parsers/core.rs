use super::*;

pub(in super::super) fn selected_modify_pass_key(app: &App) -> String {
    match app.get_modify_pass_key_text().as_str() {
        "" | "custom" => "quantize".to_string(),
        key => key.to_string(),
    }
}

pub(in super::super) fn serialize_modify_config(config: &MidiFileProcessingConfig) -> String {
    serde_json::to_string_pretty(config).unwrap_or_else(|_| "{\n  \"tool\": null\n}".into())
}

pub(in super::super) fn parse_modify_config_text(
    raw: &str,
) -> Result<MidiFileProcessingConfig, String> {
    if raw.trim().is_empty() {
        return Err("modify config is empty".into());
    }

    let value: Value =
        serde_json::from_str(raw).map_err(|error| format!("invalid JSON: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("invalid process config: {error}"))
}

pub(in super::super) fn fallback_modify_config(app: &App) -> MidiFileProcessingConfig {
    parse_modify_config_text(app.get_modify_config_text().as_str())
        .or_else(|_| parse_modify_config_text(app.get_modify_last_valid_config_text().as_str()))
        .unwrap_or_else(|_| default_modify_config(selected_modify_pass_key(app).as_str()))
}

pub(in super::super) fn set_modify_config(app: &App, config: &MidiFileProcessingConfig) {
    app.set_modify_config_text(serialize_modify_config(config).into());
    validate_modify_config(app);
}

pub(in super::super) fn option_text<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

pub(in super::super) fn toggle_text(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

pub(in super::super) fn parse_value<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Display,
{
    raw.trim()
        .parse::<T>()
        .map_err(|error| format!("invalid {label}: {error}"))
}

pub(in super::super) fn parse_optional_value<T>(raw: &str, label: &str) -> Result<Option<T>, String>
where
    T: FromStr,
    T::Err: Display,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        parse_value(trimmed, label).map(Some)
    }
}

pub(in super::super) fn parse_bool_toggle(raw: &str, label: &str) -> Result<bool, String> {
    match raw.trim() {
        "on" | "yes" | "true" | "1" => Ok(true),
        "off" | "no" | "false" | "0" => Ok(false),
        other => Err(format!("invalid {label}: {other}")),
    }
}

pub(in super::super) fn parse_serde_enum<T>(raw: &str, label: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    serde_json::from_str::<T>(&format!("\"{trimmed}\""))
        .map_err(|_| format!("invalid {label}: {trimmed}"))
}

pub(in super::super) fn format_serde_enum<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"\"".into())
        .trim_matches('"')
        .to_string()
}
