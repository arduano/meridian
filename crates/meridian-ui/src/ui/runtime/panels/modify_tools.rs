use super::*;

pub(super) fn ensure_tool_for_pass<'a>(
    config: &'a mut MidiFileProcessingConfig,
    pass_key: &str,
) -> &'a mut MidiModifierTool {
    if tool_key(&config.tool) != pass_key {
        config.tool = tool_for_pass(pass_key);
    }
    &mut config.tool
}

pub(super) fn pass_metadata(pass_key: &str) -> (&'static str, &'static str, &'static str) {
    modify_pass_policy(pass_key)
        .map(|policy| (policy.title, policy.description, policy.hint))
        .unwrap_or((
            "Custom Pipeline",
            "The current JSON does not map cleanly to one preset button.",
            "You can still run the job. Use a pass preset again if you want to reseed the draft.",
        ))
}

pub(super) fn tool_key(tool: &MidiModifierTool) -> &'static str {
    match tool {
        MidiModifierTool::RangeSelect(_) => "range_select",
        MidiModifierTool::TempoMap(_) => "tempo_map",
        MidiModifierTool::TimeWarp(_) => "time_warp",
        MidiModifierTool::ChannelRemap(_) => "channel_remap",
        MidiModifierTool::TrackRoute(_) => "track_route",
        MidiModifierTool::Program(_) => "program",
        MidiModifierTool::ControlChange(_) => "control_change",
        MidiModifierTool::PitchBend(_) => "pitch_bend",
        MidiModifierTool::VelocityMap(_) => "velocity_map",
        MidiModifierTool::ChangePpq(_) => "change_ppq",
        MidiModifierTool::ExtractTrack(_) => "extract_track",
        MidiModifierTool::NoteLength(_) => "note_length",
        MidiModifierTool::Quantize(_) => "quantize",
        MidiModifierTool::Humanize(_) => "humanize",
        MidiModifierTool::KeyMap(_) => "key_map",
        MidiModifierTool::MetaText(_) => "meta_text",
        MidiModifierTool::Sysex(_) => "sysex",
        MidiModifierTool::SharedMetadataTrack(_) => "shared_metadata_track",
    }
}

pub(super) fn tool_for_pass(pass_key: &str) -> MidiModifierTool {
    modify_pass_policy(pass_key)
        .map(|policy| (policy.default_tool)())
        .unwrap_or_else(|| {
            MidiModifierTool::Quantize(QuantizeTool {
                rounding_ticks: 120,
                mode: meridian_core::midi::QuantizeMode::NoteStartOnly,
            })
        })
}

pub(super) fn default_modify_config(pass_key: &str) -> MidiFileProcessingConfig {
    MidiFileProcessingConfig {
        tool: tool_for_pass(pass_key),
    }
}

pub(in super::super) fn load_modify_pass_into_app(app: &App, pass_key: &str) {
    let config = MidiFileProcessingConfig {
        tool: tool_for_pass(pass_key),
    };
    set_modify_config(app, &config);
}

pub(super) fn sync_modify_pass_metadata(app: &App, pass_key: &str) {
    let (title, description, hint) = pass_metadata(pass_key);
    app.set_modify_pass_key_text(pass_key.into());
    app.set_modify_pass_title_text(title.into());
    app.set_modify_pass_description_text(description.into());
    app.set_modify_pass_hint_text(hint.into());
}

pub(super) fn sync_modify_pass_metadata_from_config(app: &App, config: &MidiFileProcessingConfig) {
    sync_modify_pass_metadata(app, tool_key(&config.tool));
}

pub(in super::super) fn validate_modify_config(app: &App) {
    match parse_modify_config_text(app.get_modify_config_text().as_str()) {
        Ok(config) => {
            app.set_modify_last_valid_config_text(serialize_modify_config(&config).into());
            sync_modify_pass_metadata_from_config(app, &config);
            sync_modify_pass_controls(app, &config);
            app.set_modify_config_valid(true);
            let status = format!("Valid JSON. Tool `{}` configured.", tool_key(&config.tool));
            app.set_modify_config_status_text(status.into());
        }
        Err(error) => {
            app.set_modify_config_valid(false);
            app.set_modify_config_status_text(format!("JSON parse error: {error}").into());
        }
    }
}
