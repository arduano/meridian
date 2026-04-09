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
    match pass_key {
        "range_select" => (
            "Range Select",
            "Limit later tools to a subset of tracks, channels, keys, ticks, velocities, or event kinds.",
            "Useful as the first tool in a multi-pass pipeline when you only want to touch one region of the file.",
        ),
        "tempo_map" => (
            "Tempo Map",
            "Flatten, scale, or fully replace tempo events while leaving the rest of the file intact.",
            "The preset starts with a no-op tempo scale so you can switch modes or edit values in place.",
        ),
        "time_warp" => (
            "Time Warp",
            "Remap absolute ticks through custom control points for rubato fixes or structural timing edits.",
            "Edit the `points` array with `source_tick` and `dest_tick` values in ascending order.",
        ),
        "channel_remap" => (
            "Channel Remap",
            "Move note and controller data between MIDI channels without rebuilding the file by hand.",
            "Add mapping entries like `{ \"from\": 0, \"to\": 1 }` to retarget channels.",
        ),
        "track_route" => (
            "Track Route",
            "Collapse, split, or explicitly remap tracks before the file is written back out.",
            "Switch the `mode` field to `collapse_all`, `split_by_channel`, or `map`.",
        ),
        "program" => (
            "Program",
            "Force startup programs or strip later program changes.",
            "Use this when a synth or export target needs deterministic instrument assignments.",
        ),
        "control_change" => (
            "Control Change",
            "Strip, remap, scale, or inject controller values at the start of playback.",
            "Good for fixing sustain pedals, volume curves, expression, or target-device controller layouts.",
        ),
        "pitch_bend" => (
            "Pitch Bend",
            "Scale, offset, clamp, or strip pitch-bend data without touching note timing.",
            "The preset keeps bends intact with full-range limits so you can dial in corrections safely.",
        ),
        "velocity_map" => (
            "Velocity Map",
            "Re-shape note-on velocities with a scale, gamma curve, or custom polyline.",
            "Start with a simple scale, or replace the tool variant in JSON for more detailed curves.",
        ),
        "note_length" => (
            "Note Length",
            "Clamp, scale, or replace note durations across the selected note set.",
            "Combine with `range_select` if only part of the arrangement should be resized.",
        ),
        "quantize" => (
            "Quantize",
            "Snap note starts, note ends, or all events to a tick grid.",
            "The default preset uses 120-tick rounding; choose the mode that fits the file.",
        ),
        "humanize" => (
            "Humanize",
            "Add deterministic timing, duration, and velocity variation using a reproducible seed.",
            "Use small values first. The preset is intentionally gentle so you can hear the change without wrecking alignment.",
        ),
        "key_map" => (
            "Key Map",
            "Remap source notes to new pitches, fold them into a target range, or drop unmapped notes.",
            "Useful for keyboard reductions, drum remaps, and narrowing orchestral parts into a playable register.",
        ),
        "meta_text" => (
            "Meta Text",
            "Keep only the metadata text kinds you want without touching musical events.",
            "Helpful when preparing clean delivery files or standardizing merged project metadata.",
        ),
        "sysex" => (
            "SysEx",
            "Strip SysEx entirely or prepend specific setup messages at the start of the file.",
            "Use raw byte arrays in decimal form inside `prepend`, for example `[[67,16,76]]`.",
        ),
        "shared_metadata_track" => (
            "Shared Metadata Track",
            "Move shared conductor and setup events into one track, optionally stripping redundant state.",
            "Useful when you want a clean shared metadata lane before export or downstream conversion.",
        ),
        "change_ppq" => (
            "Change PPQ",
            "Resample every tick in the file to a new pulses-per-quarter-note resolution.",
            "Common values: 96, 120, 240, 480, 960. This changes tick density, not musical timing.",
        ),
        "extract_track" => (
            "Extract Track",
            "Write one source track into a new single-track MIDI file.",
            "Use the zero-based track index shown in the source-file stats strip above.",
        ),
        _ => (
            "Custom Pipeline",
            "The current JSON does not map cleanly to one preset button.",
            "You can still run the job. Use a pass preset again if you want to reseed the draft.",
        ),
    }
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
    match pass_key {
        "range_select" => MidiModifierTool::RangeSelect(RangeSelectTool::default()),
        "tempo_map" => MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor: 1.0 }),
        "time_warp" => MidiModifierTool::TimeWarp(TimeWarpTool::default()),
        "channel_remap" => MidiModifierTool::ChannelRemap(ChannelRemapTool::default()),
        "track_route" => MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll),
        "program" => MidiModifierTool::Program(ProgramTool {
            force_program: None,
            strip_program_changes: false,
            startup_programs: Vec::new(),
        }),
        "control_change" => MidiModifierTool::ControlChange(ControlChangeTool::default()),
        "pitch_bend" => MidiModifierTool::PitchBend(PitchBendTool {
            strip: false,
            scale: 1.0,
            offset: 0,
            min_bend: -8192,
            max_bend: 8191,
        }),
        "velocity_map" => MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 }),
        "note_length" => MidiModifierTool::NoteLength(NoteLengthTool {
            min_ticks: None,
            max_ticks: None,
            scale: Some(1.0),
            fixed_ticks: None,
        }),
        "quantize" => MidiModifierTool::Quantize(QuantizeTool {
            rounding_ticks: 120,
            mode: meridian_core::midi::QuantizeMode::NoteStartOnly,
        }),
        "humanize" => MidiModifierTool::Humanize(HumanizeTool {
            start_jitter: 8,
            length_jitter: 6,
            velocity_jitter: 5,
            seed: 1,
            collision_mode: meridian_core::midi::HumanizeCollisionMode::Stable,
        }),
        "key_map" => MidiModifierTool::KeyMap(KeyMapTool::default()),
        "meta_text" => MidiModifierTool::MetaText(MetaTextTool::default()),
        "sysex" => MidiModifierTool::Sysex(SysexTool::default()),
        "shared_metadata_track" => MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
            destination: SharedMetadataTrackDestination::CreateNew,
            strip_redundant_events: false,
            move_tempo_events: true,
            move_time_signatures: true,
            move_key_signatures: true,
            move_text_events: false,
            move_unknown_meta_events: false,
            move_channel_prefix_events: false,
            move_midi_port_events: false,
            move_control_change_events: false,
            move_program_change_events: false,
            move_pitch_bend_events: false,
            move_channel_pressure_events: false,
        }),
        "change_ppq" => MidiModifierTool::ChangePpq(ChangePpqTool { ppq: 480 }),
        "extract_track" => MidiModifierTool::ExtractTrack(ExtractTrackTool { track_index: 0 }),
        _ => MidiModifierTool::Quantize(QuantizeTool {
            rounding_ticks: 120,
            mode: meridian_core::midi::QuantizeMode::NoteStartOnly,
        }),
    }
}

pub(super) fn default_modify_config(pass_key: &str) -> MidiFileProcessingConfig {
    MidiFileProcessingConfig {
        tool: tool_for_pass(pass_key),
    }
}

pub(super) fn load_modify_pass_into_app(app: &App, pass_key: &str) {
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

pub(super) fn validate_modify_config(app: &App) {
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
