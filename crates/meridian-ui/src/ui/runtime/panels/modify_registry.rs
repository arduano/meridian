use super::*;

pub(super) struct ModifyPassPolicy {
    pub key: &'static str,
    pub field_prefix: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
    pub default_tool: fn() -> MidiModifierTool,
    pub sync_controls: fn(&App, &MidiFileProcessingConfig),
    pub update_control:
        fn(&App, &mut MidiFileProcessingConfig, &str, &str) -> Result<(), String>,
}

fn range_select_default_tool() -> MidiModifierTool {
    MidiModifierTool::RangeSelect(RangeSelectTool::default())
}

fn tempo_map_default_tool() -> MidiModifierTool {
    MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor: 1.0 })
}

fn time_warp_default_tool() -> MidiModifierTool {
    MidiModifierTool::TimeWarp(TimeWarpTool::default())
}

fn channel_remap_default_tool() -> MidiModifierTool {
    MidiModifierTool::ChannelRemap(ChannelRemapTool::default())
}

fn track_route_default_tool() -> MidiModifierTool {
    MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll)
}

fn program_default_tool() -> MidiModifierTool {
    MidiModifierTool::Program(ProgramTool {
        force_program: None,
        strip_program_changes: false,
        startup_programs: Vec::new(),
    })
}

fn control_change_default_tool() -> MidiModifierTool {
    MidiModifierTool::ControlChange(ControlChangeTool::default())
}

fn pitch_bend_default_tool() -> MidiModifierTool {
    MidiModifierTool::PitchBend(PitchBendTool {
        strip: false,
        scale: 1.0,
        offset: 0,
        min_bend: -8192,
        max_bend: 8191,
    })
}

fn velocity_map_default_tool() -> MidiModifierTool {
    MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 })
}

fn note_length_default_tool() -> MidiModifierTool {
    MidiModifierTool::NoteLength(NoteLengthTool {
        min_ticks: None,
        max_ticks: None,
        scale: Some(1.0),
        fixed_ticks: None,
    })
}

fn quantize_default_tool() -> MidiModifierTool {
    MidiModifierTool::Quantize(QuantizeTool {
        rounding_ticks: 120,
        mode: meridian_core::midi::QuantizeMode::NoteStartOnly,
    })
}

fn humanize_default_tool() -> MidiModifierTool {
    MidiModifierTool::Humanize(HumanizeTool {
        start_jitter: 8,
        length_jitter: 6,
        velocity_jitter: 5,
        seed: 1,
        collision_mode: meridian_core::midi::HumanizeCollisionMode::Stable,
    })
}

fn key_map_default_tool() -> MidiModifierTool {
    MidiModifierTool::KeyMap(KeyMapTool::default())
}

fn meta_text_default_tool() -> MidiModifierTool {
    MidiModifierTool::MetaText(MetaTextTool::default())
}

fn sysex_default_tool() -> MidiModifierTool {
    MidiModifierTool::Sysex(SysexTool::default())
}

fn shared_metadata_track_default_tool() -> MidiModifierTool {
    MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
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
    })
}

fn change_ppq_default_tool() -> MidiModifierTool {
    MidiModifierTool::ChangePpq(ChangePpqTool { ppq: 480 })
}

fn extract_track_default_tool() -> MidiModifierTool {
    MidiModifierTool::ExtractTrack(ExtractTrackTool { track_index: 0 })
}

pub(super) const MODIFY_PASS_POLICIES: &[ModifyPassPolicy] = &[
    ModifyPassPolicy {
        key: "range_select",
        field_prefix: "range",
        title: "Range Select",
        description: "Limit later tools to a subset of tracks, channels, keys, ticks, velocities, or event kinds.",
        hint: "Useful as the first tool in a multi-pass pipeline when you only want to touch one region of the file.",
        default_tool: range_select_default_tool,
        sync_controls: sync_range_select_controls,
        update_control: update_range_select_control,
    },
    ModifyPassPolicy {
        key: "tempo_map",
        field_prefix: "tempo",
        title: "Tempo Map",
        description: "Flatten, scale, or fully replace tempo events while leaving the rest of the file intact.",
        hint: "The preset starts with a no-op tempo scale so you can switch modes or edit values in place.",
        default_tool: tempo_map_default_tool,
        sync_controls: sync_tempo_map_controls,
        update_control: update_tempo_map_control,
    },
    ModifyPassPolicy {
        key: "time_warp",
        field_prefix: "time_warp",
        title: "Time Warp",
        description: "Remap absolute ticks through custom control points for rubato fixes or structural timing edits.",
        hint: "Edit the `points` array with `source_tick` and `dest_tick` values in ascending order.",
        default_tool: time_warp_default_tool,
        sync_controls: sync_time_warp_controls,
        update_control: update_time_warp_control,
    },
    ModifyPassPolicy {
        key: "channel_remap",
        field_prefix: "channel_remap",
        title: "Channel Remap",
        description: "Move note and controller data between MIDI channels without rebuilding the file by hand.",
        hint: "Add mapping entries like `{ \"from\": 0, \"to\": 1 }` to retarget channels.",
        default_tool: channel_remap_default_tool,
        sync_controls: sync_channel_remap_controls,
        update_control: update_channel_remap_control,
    },
    ModifyPassPolicy {
        key: "track_route",
        field_prefix: "track_route",
        title: "Track Route",
        description: "Collapse, split, or explicitly remap tracks before the file is written back out.",
        hint: "Switch the `mode` field to `collapse_all`, `split_by_channel`, or `map`.",
        default_tool: track_route_default_tool,
        sync_controls: sync_track_route_controls,
        update_control: update_track_route_control,
    },
    ModifyPassPolicy {
        key: "program",
        field_prefix: "program",
        title: "Program",
        description: "Force startup programs or strip later program changes.",
        hint: "Use this when a synth or export target needs deterministic instrument assignments.",
        default_tool: program_default_tool,
        sync_controls: sync_program_controls,
        update_control: update_program_control,
    },
    ModifyPassPolicy {
        key: "control_change",
        field_prefix: "control",
        title: "Control Change",
        description: "Strip, remap, scale, or inject controller values at the start of playback.",
        hint: "Good for fixing sustain pedals, volume curves, expression, or target-device controller layouts.",
        default_tool: control_change_default_tool,
        sync_controls: sync_control_change_controls,
        update_control: update_control_change_control,
    },
    ModifyPassPolicy {
        key: "pitch_bend",
        field_prefix: "pitch",
        title: "Pitch Bend",
        description: "Scale, offset, clamp, or strip pitch-bend data without touching note timing.",
        hint: "The preset keeps bends intact with full-range limits so you can dial in corrections safely.",
        default_tool: pitch_bend_default_tool,
        sync_controls: sync_pitch_bend_controls,
        update_control: update_pitch_bend_control,
    },
    ModifyPassPolicy {
        key: "velocity_map",
        field_prefix: "velocity",
        title: "Velocity Map",
        description: "Re-shape note-on velocities with a scale, gamma curve, or custom polyline.",
        hint: "Start with a simple scale, or replace the tool variant in JSON for more detailed curves.",
        default_tool: velocity_map_default_tool,
        sync_controls: sync_velocity_map_controls,
        update_control: update_velocity_map_control,
    },
    ModifyPassPolicy {
        key: "note_length",
        field_prefix: "note_length",
        title: "Note Length",
        description: "Clamp, scale, or replace note durations across the selected note set.",
        hint: "Combine with `range_select` if only part of the arrangement should be resized.",
        default_tool: note_length_default_tool,
        sync_controls: sync_note_length_controls,
        update_control: update_note_length_control,
    },
    ModifyPassPolicy {
        key: "quantize",
        field_prefix: "quantize",
        title: "Quantize",
        description: "Snap note starts, note ends, or all events to a tick grid.",
        hint: "The default preset uses 120-tick rounding; choose the mode that fits the file.",
        default_tool: quantize_default_tool,
        sync_controls: sync_quantize_controls,
        update_control: update_quantize_control,
    },
    ModifyPassPolicy {
        key: "humanize",
        field_prefix: "humanize",
        title: "Humanize",
        description: "Add deterministic timing, duration, and velocity variation using a reproducible seed.",
        hint: "Use small values first. The preset is intentionally gentle so you can hear the change without wrecking alignment.",
        default_tool: humanize_default_tool,
        sync_controls: sync_humanize_controls,
        update_control: update_humanize_control,
    },
    ModifyPassPolicy {
        key: "key_map",
        field_prefix: "key_map",
        title: "Key Map",
        description: "Remap source notes to new pitches, fold them into a target range, or drop unmapped notes.",
        hint: "Useful for keyboard reductions, drum remaps, and narrowing orchestral parts into a playable register.",
        default_tool: key_map_default_tool,
        sync_controls: sync_key_map_controls,
        update_control: update_key_map_control,
    },
    ModifyPassPolicy {
        key: "meta_text",
        field_prefix: "meta",
        title: "Meta Text",
        description: "Keep only the metadata text kinds you want without touching musical events.",
        hint: "Helpful when preparing clean delivery files or standardizing merged project metadata.",
        default_tool: meta_text_default_tool,
        sync_controls: sync_meta_text_controls,
        update_control: update_meta_text_control,
    },
    ModifyPassPolicy {
        key: "sysex",
        field_prefix: "sysex",
        title: "SysEx",
        description: "Strip SysEx entirely or prepend specific setup messages at the start of the file.",
        hint: "Use raw byte arrays in decimal form inside `prepend`, for example `[[67,16,76]]`.",
        default_tool: sysex_default_tool,
        sync_controls: sync_sysex_controls,
        update_control: update_sysex_control,
    },
    ModifyPassPolicy {
        key: "shared_metadata_track",
        field_prefix: "shared_metadata",
        title: "Shared Metadata Track",
        description: "Move shared conductor and setup events into one track, optionally stripping redundant state.",
        hint: "Useful when you want a clean shared metadata lane before export or downstream conversion.",
        default_tool: shared_metadata_track_default_tool,
        sync_controls: sync_shared_metadata_track_controls,
        update_control: update_shared_metadata_track_control,
    },
    ModifyPassPolicy {
        key: "change_ppq",
        field_prefix: "change_ppq",
        title: "Change PPQ",
        description: "Resample every tick in the file to a new pulses-per-quarter-note resolution.",
        hint: "Common values: 96, 120, 240, 480, 960. This changes tick density, not musical timing.",
        default_tool: change_ppq_default_tool,
        sync_controls: sync_change_ppq_controls,
        update_control: update_change_ppq_control,
    },
    ModifyPassPolicy {
        key: "extract_track",
        field_prefix: "extract_track",
        title: "Extract Track",
        description: "Write one source track into a new single-track MIDI file.",
        hint: "Use the zero-based track index shown in the source-file stats strip above.",
        default_tool: extract_track_default_tool,
        sync_controls: sync_extract_track_controls,
        update_control: update_extract_track_control,
    },
];

pub(super) fn modify_pass_policy(pass_key: &str) -> Option<&'static ModifyPassPolicy> {
    MODIFY_PASS_POLICIES.iter().find(|policy| policy.key == pass_key)
}

pub(super) fn modify_pass_policy_for_field_prefix(
    field_prefix: &str,
) -> Option<&'static ModifyPassPolicy> {
    MODIFY_PASS_POLICIES
        .iter()
        .find(|policy| policy.field_prefix == field_prefix)
}

pub(super) fn modify_pass_keys() -> &'static [&'static str] {
    &[
        "range_select",
        "tempo_map",
        "time_warp",
        "channel_remap",
        "track_route",
        "program",
        "control_change",
        "pitch_bend",
        "velocity_map",
        "note_length",
        "quantize",
        "humanize",
        "key_map",
        "meta_text",
        "sysex",
        "shared_metadata_track",
        "change_ppq",
        "extract_track",
    ]
}

pub(super) fn sync_range_select_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::RangeSelect(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_range_track_select_text(option_text(tool.track_select).into());
        app.set_modify_range_start_ticks_text(format_notes(tool.start_ticks, ppq).into());
        app.set_modify_range_end_ticks_text(format_notes(tool.end_ticks, ppq).into());
        app.set_modify_range_offset_ticks_text(
            tool.offset_ticks
                .map(|t| format_notes(t, ppq))
                .unwrap_or_default()
                .into(),
        );
        app.set_modify_range_preserve_system_text(toggle_text(tool.preserve_system_events).into());
        app.set_modify_range_edge_behavior_text(format_serde_enum(&tool.edge_behavior).into());
    }
}

pub(super) fn update_range_select_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "track_select" | "offset_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(config, "range_select")
            {
                if field == "track_select" {
                    tool.track_select = parse_optional_value(value, "range track")?;
                } else {
                    let ppq = get_ppq(app);
                    tool.offset_ticks = parse_optional_notes(value, ppq, "range offset")?;
                }
            }
        }
        "preserve_system_events" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(config, "range_select")
            {
                tool.preserve_system_events = parse_bool_toggle(value, "preserve system events")?;
            }
        }
        "edge_behavior" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(config, "range_select")
            {
                tool.edge_behavior = parse_serde_enum(value, "range edge behavior")?;
            }
        }
        "start_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(config, "range_select")
            {
                let ppq = get_ppq(app);
                tool.start_ticks = parse_notes(value, ppq, "range start")?;
            }
        }
        "end_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(config, "range_select")
            {
                let ppq = get_ppq(app);
                tool.end_ticks = parse_notes(value, ppq, "range end")?;
            }
        }
        other => return Err(format!("unknown range control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_tempo_map_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::TempoMap(tool) = &config.tool {
        match tool {
            TempoMapTool::Flatten { tempo } => {
                app.set_modify_tempo_mode_text("flatten".into());
                app.set_modify_tempo_flatten_tempo_text(tempo.to_string().into());
            }
            TempoMapTool::ScaleBpm { factor } => {
                app.set_modify_tempo_mode_text("scale_bpm".into());
                app.set_modify_tempo_scale_factor_text(factor.to_string().into());
            }
            TempoMapTool::Replace {
                points,
                destination,
            } => {
                let ppq = get_ppq(app);
                app.set_modify_tempo_mode_text("replace".into());
                app.set_modify_tempo_replace_points_text(
                    format_tempo_points_notes(points, ppq).into(),
                );
                app.set_modify_tempo_scale_factor_text(format_serde_enum(destination).into());
            }
        }
    }
}

pub(super) fn update_tempo_map_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "mode" => {
            let tool = match value.trim() {
                "flatten" => MidiModifierTool::TempoMap(TempoMapTool::Flatten { tempo: 500000 }),
                "scale_bpm" => MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor: 1.0 }),
                "replace" => MidiModifierTool::TempoMap(TempoMapTool::Replace {
                    points: vec![],
                    destination: TempoMapDestination::InjectIntoFirstTrack,
                }),
                other => return Err(format!("invalid tempo mode: {other}")),
            };
            config.tool = tool;
        }
        "flatten_tempo" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Flatten { tempo }) =
                ensure_tool_for_pass(config, "tempo_map")
            {
                *tempo = parse_value(value, "flatten tempo")?;
            }
        }
        "scale_factor" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor }) =
                ensure_tool_for_pass(config, "tempo_map")
            {
                *factor = parse_value(value, "scale factor")?;
            }
        }
        "replace_points" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Replace { points, .. }) =
                ensure_tool_for_pass(config, "tempo_map")
            {
                let ppq = get_ppq(app);
                *points = parse_tempo_points_notes(value, ppq)?;
            }
        }
        "destination" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Replace { destination, .. }) =
                ensure_tool_for_pass(config, "tempo_map")
            {
                *destination = parse_serde_enum(value, "tempo destination")?;
            }
        }
        other => return Err(format!("unknown tempo control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_time_warp_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::TimeWarp(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_time_warp_points_text(format_time_warp_points_notes(&tool.points, ppq).into());
    }
}

pub(super) fn update_time_warp_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "points" => {
            if let MidiModifierTool::TimeWarp(tool) = ensure_tool_for_pass(config, "time_warp") {
                let ppq = get_ppq(app);
                tool.points = parse_time_warp_points_notes(value, ppq)?;
            }
        }
        other => return Err(format!("unknown time-warp control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_channel_remap_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ChannelRemap(tool) = &config.tool {
        app.set_modify_channel_remap_mappings_text(format_channel_mappings(&tool.mappings).into());
    }
}

pub(super) fn update_channel_remap_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "mappings" => {
            if let MidiModifierTool::ChannelRemap(tool) =
                ensure_tool_for_pass(config, "channel_remap")
            {
                tool.mappings = parse_channel_mappings(value)?;
            }
        }
        other => return Err(format!("unknown channel remap control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_track_route_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::TrackRoute(tool) = &config.tool {
        match tool {
            TrackRouteTool::CollapseAll => {
                app.set_modify_track_route_mode_text("collapse_all".into());
            }
            TrackRouteTool::SplitByChannel => {
                app.set_modify_track_route_mode_text("split_by_channel".into());
            }
            TrackRouteTool::Map { mappings } => {
                app.set_modify_track_route_mode_text("map".into());
                app.set_modify_track_route_mappings_text(format_track_mappings(mappings).into());
            }
        }
    }
}

pub(super) fn update_track_route_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "mode" => {
            if let MidiModifierTool::TrackRoute(tool) = ensure_tool_for_pass(config, "track_route")
            {
                *tool = match value.trim() {
                    "collapse_all" => TrackRouteTool::CollapseAll,
                    "split_by_channel" => TrackRouteTool::SplitByChannel,
                    "map" => TrackRouteTool::Map { mappings: vec![] },
                    other => return Err(format!("invalid track route mode: {other}")),
                };
            }
        }
        "mappings" => {
            if let MidiModifierTool::TrackRoute(TrackRouteTool::Map { mappings }) =
                ensure_tool_for_pass(config, "track_route")
            {
                *mappings = parse_track_mappings(value)?;
            }
        }
        other => return Err(format!("unknown track route control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_program_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Program(tool) = &config.tool {
        app.set_modify_program_force_program_text(option_text(tool.force_program).into());
        app.set_modify_program_strip_changes_text(toggle_text(tool.strip_program_changes).into());
        app.set_modify_program_startup_programs_text(
            format_channel_programs(&tool.startup_programs).into(),
        );
    }
}

pub(super) fn update_program_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "force_program" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(config, "program") {
                tool.force_program = parse_optional_value(value, "program number")?;
            }
        }
        "strip_program_changes" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(config, "program") {
                tool.strip_program_changes = parse_bool_toggle(value, "program strip toggle")?;
            }
        }
        "startup_programs" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(config, "program") {
                tool.startup_programs = parse_channel_programs(value)?;
            }
        }
        other => return Err(format!("unknown program control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_control_change_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ControlChange(tool) = &config.tool {
        app.set_modify_control_strip_controllers_text(format_u8_list(&tool.strip_controllers).into());
        app.set_modify_control_remap_controllers_text(
            format_controller_mappings(&tool.remap_controllers).into(),
        );
        app.set_modify_control_scale_controllers_text(
            format_controller_scales(&tool.scale_controllers).into(),
        );
        app.set_modify_control_inject_start_text(format_control_values(&tool.inject_start).into());
    }
}

pub(super) fn update_control_change_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "strip_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(config, "control_change")
            {
                tool.strip_controllers = parse_u8_list(value, "strip controller")?;
            }
        }
        "remap_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(config, "control_change")
            {
                tool.remap_controllers = parse_controller_mappings(value)?;
            }
        }
        "scale_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(config, "control_change")
            {
                tool.scale_controllers = parse_controller_scales(value)?;
            }
        }
        "inject_start" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(config, "control_change")
            {
                tool.inject_start = parse_control_values(value)?;
            }
        }
        other => return Err(format!("unknown control change control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_pitch_bend_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::PitchBend(tool) = &config.tool {
        app.set_modify_pitch_strip_text(toggle_text(tool.strip).into());
        app.set_modify_pitch_scale_text(tool.scale.to_string().into());
        app.set_modify_pitch_offset_text(tool.offset.to_string().into());
        app.set_modify_pitch_min_bend_text(tool.min_bend.to_string().into());
        app.set_modify_pitch_max_bend_text(tool.max_bend.to_string().into());
    }
}

pub(super) fn update_pitch_bend_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "strip" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(config, "pitch_bend")
            {
                tool.strip = parse_bool_toggle(value, "pitch bend strip toggle")?;
            }
        }
        "scale" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(config, "pitch_bend")
            {
                tool.scale = parse_value(value, "pitch bend scale")?;
            }
        }
        "offset" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(config, "pitch_bend")
            {
                tool.offset = parse_value(value, "pitch bend offset")?;
            }
        }
        "min_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(config, "pitch_bend")
            {
                tool.min_bend = parse_value(value, "pitch bend min")?;
            }
        }
        "max_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(config, "pitch_bend")
            {
                tool.max_bend = parse_value(value, "pitch bend max")?;
            }
        }
        other => return Err(format!("unknown pitch bend control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_velocity_map_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::VelocityMap(tool) = &config.tool {
        match tool {
            VelocityMapTool::Scale { scale } => {
                app.set_modify_velocity_mode_text("scale".into());
                app.set_modify_velocity_scale_text(scale.to_string().into());
            }
            VelocityMapTool::Gamma { gamma } => {
                app.set_modify_velocity_mode_text("gamma".into());
                app.set_modify_velocity_gamma_text(gamma.to_string().into());
            }
            VelocityMapTool::Polyline { points } => {
                app.set_modify_velocity_mode_text("polyline".into());
                app.set_modify_velocity_points_text(format_velocity_points(points).into());
            }
        }
    }
}

pub(super) fn update_velocity_map_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "mode" => {
            let tool = match value.trim() {
                "scale" => MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 }),
                "gamma" => MidiModifierTool::VelocityMap(VelocityMapTool::Gamma { gamma: 1.0 }),
                "polyline" => MidiModifierTool::VelocityMap(VelocityMapTool::Polyline {
                    points: vec![],
                }),
                other => return Err(format!("invalid velocity mode: {other}")),
            };
            config.tool = tool;
        }
        "scale" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale }) =
                ensure_tool_for_pass(config, "velocity_map")
            {
                *scale = parse_value(value, "velocity scale")?;
            }
        }
        "gamma" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Gamma { gamma }) =
                ensure_tool_for_pass(config, "velocity_map")
            {
                *gamma = parse_value(value, "velocity gamma")?;
            }
        }
        "points" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Polyline { points }) =
                ensure_tool_for_pass(config, "velocity_map")
            {
                *points = parse_velocity_points(value)?;
            }
        }
        other => return Err(format!("unknown velocity control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_note_length_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::NoteLength(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_note_length_min_ticks_text(
            format_option_notes(tool.min_ticks, ppq).into(),
        );
        app.set_modify_note_length_max_ticks_text(
            format_option_notes(tool.max_ticks, ppq).into(),
        );
        app.set_modify_note_length_scale_text(option_text(tool.scale).into());
        app.set_modify_note_length_fixed_ticks_text(
            format_option_notes(tool.fixed_ticks, ppq).into(),
        );
    }
}

pub(super) fn update_note_length_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "min_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.min_ticks = parse_optional_notes(value, ppq, "note length min")?;
            }
        }
        "max_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.max_ticks = parse_optional_notes(value, ppq, "note length max")?;
            }
        }
        "scale" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(config, "note_length")
            {
                tool.scale = parse_optional_value(value, "note length scale")?;
            }
        }
        "fixed_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.fixed_ticks = parse_optional_notes(value, ppq, "note length fixed")?;
            }
        }
        other => return Err(format!("unknown note length control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_quantize_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Quantize(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_quantize_grid_ticks_text(
            format_notes(tool.rounding_ticks as u64, ppq).into(),
        );
        app.set_modify_quantize_mode_text(format_serde_enum(&tool.mode).into());
    }
}

pub(super) fn update_quantize_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "grid_ticks" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(config, "quantize") {
                let ppq = get_ppq(app);
                tool.rounding_ticks = parse_notes(value, ppq, "quantize grid")?;
            }
        }
        "mode" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(config, "quantize") {
                tool.mode = parse_serde_enum(value, "quantize mode")?;
            }
        }
        other => return Err(format!("unknown quantize control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_humanize_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Humanize(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_humanize_start_jitter_text(
            format_notes(tool.start_jitter as u64, ppq).into(),
        );
        app.set_modify_humanize_length_jitter_text(
            format_notes(tool.length_jitter as u64, ppq).into(),
        );
        app.set_modify_humanize_velocity_jitter_text(tool.velocity_jitter.to_string().into());
        app.set_modify_humanize_seed_text(tool.seed.to_string().into());
        app.set_modify_humanize_collision_mode_text(
            format_serde_enum(&tool.collision_mode).into(),
        );
    }
}

pub(super) fn update_humanize_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "start_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(config, "humanize") {
                let ppq = get_ppq(app);
                tool.start_jitter = parse_notes(value, ppq, "humanize start jitter")? as i64;
            }
        }
        "length_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(config, "humanize") {
                let ppq = get_ppq(app);
                tool.length_jitter = parse_notes(value, ppq, "humanize length jitter")? as i64;
            }
        }
        "velocity_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(config, "humanize") {
                tool.velocity_jitter = parse_value(value, "humanize velocity jitter")?;
            }
        }
        "seed" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(config, "humanize") {
                tool.seed = parse_value(value, "humanize seed")?;
            }
        }
        "collision_mode" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(config, "humanize") {
                tool.collision_mode = parse_serde_enum(value, "humanize collision mode")?;
            }
        }
        other => return Err(format!("unknown humanize control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_key_map_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::KeyMap(tool) = &config.tool {
        app.set_modify_key_map_mappings_text(format_key_mappings(&tool.mappings).into());
        app.set_modify_key_map_fold_range_text(format_key_range(&tool.fold_to_range).into());
        app.set_modify_key_map_drop_unmapped_text(toggle_text(tool.drop_unmapped).into());
    }
}

pub(super) fn update_key_map_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "mappings" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(config, "key_map") {
                tool.mappings = parse_key_mappings(value)?;
            }
        }
        "fold_range" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(config, "key_map") {
                tool.fold_to_range = parse_key_range(value)?;
            }
        }
        "drop_unmapped" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(config, "key_map") {
                tool.drop_unmapped = parse_bool_toggle(value, "drop-unmapped toggle")?;
            }
        }
        other => return Err(format!("unknown key map control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_meta_text_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::MetaText(tool) = &config.tool {
        app.set_modify_meta_keep_kinds_text(format_text_kinds(&tool.keep_kinds).into());
    }
}

pub(super) fn update_meta_text_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "keep_kinds" => {
            if let MidiModifierTool::MetaText(tool) = ensure_tool_for_pass(config, "meta_text") {
                tool.keep_kinds = parse_text_kinds(value)?;
            }
        }
        other => return Err(format!("unknown meta text control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_sysex_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Sysex(tool) = &config.tool {
        app.set_modify_sysex_strip_all_text(toggle_text(tool.strip_all).into());
        app.set_modify_sysex_prepend_text(format_sysex_messages(&tool.prepend).into());
    }
}

pub(super) fn update_sysex_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "strip_all" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(config, "sysex") {
                tool.strip_all = parse_bool_toggle(value, "sysex strip-all toggle")?;
            }
        }
        "prepend" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(config, "sysex") {
                tool.prepend = parse_sysex_messages(value)?;
            }
        }
        other => return Err(format!("unknown sysex control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_shared_metadata_track_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::SharedMetadataTrack(tool) = &config.tool {
        app.set_modify_shared_dest_mode_text(
            match &tool.destination {
                SharedMetadataTrackDestination::CreateNew => "create_new",
                SharedMetadataTrackDestination::InsertInto { .. } => "insert_into",
            }
            .into(),
        );
        if let SharedMetadataTrackDestination::InsertInto { track_index } = &tool.destination {
            app.set_modify_shared_dest_track_text(track_index.to_string().into());
        }
        app.set_modify_shared_strip_redundant_text(
            toggle_text(tool.strip_redundant_events).into(),
        );
        app.set_modify_shared_move_tempo_text(toggle_text(tool.move_tempo_events).into());
        app.set_modify_shared_move_time_sig_text(toggle_text(tool.move_time_signatures).into());
        app.set_modify_shared_move_key_sig_text(toggle_text(tool.move_key_signatures).into());
        app.set_modify_shared_move_text_text(toggle_text(tool.move_text_events).into());
        app.set_modify_shared_move_unknown_meta_text(
            toggle_text(tool.move_unknown_meta_events).into(),
        );
        app.set_modify_shared_move_channel_prefix_text(
            toggle_text(tool.move_channel_prefix_events).into(),
        );
        app.set_modify_shared_move_midi_port_text(toggle_text(tool.move_midi_port_events).into());
        app.set_modify_shared_move_cc_text(toggle_text(tool.move_control_change_events).into());
        app.set_modify_shared_move_program_text(toggle_text(tool.move_program_change_events).into());
        app.set_modify_shared_move_pitch_bend_text(toggle_text(tool.move_pitch_bend_events).into());
        app.set_modify_shared_move_channel_pressure_text(
            toggle_text(tool.move_channel_pressure_events).into(),
        );
    }
}

pub(super) fn update_shared_metadata_track_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "dest_mode" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.destination = match value.trim() {
                    "create_new" => SharedMetadataTrackDestination::CreateNew,
                    "insert_into" => {
                        SharedMetadataTrackDestination::InsertInto { track_index: 0 }
                    }
                    other => return Err(format!("invalid destination mode: {other}")),
                };
            }
        }
        "dest_track" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                let idx = parse_value(value, "destination track index")?;
                tool.destination = SharedMetadataTrackDestination::InsertInto { track_index: idx };
            }
        }
        "strip_redundant" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.strip_redundant_events =
                    parse_bool_toggle(value, "strip redundant toggle")?;
            }
        }
        "move_tempo" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_tempo_events = parse_bool_toggle(value, "move tempo toggle")?;
            }
        }
        "move_time_sig" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_time_signatures =
                    parse_bool_toggle(value, "move time signatures toggle")?;
            }
        }
        "move_key_sig" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_key_signatures =
                    parse_bool_toggle(value, "move key signatures toggle")?;
            }
        }
        "move_text" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_text_events = parse_bool_toggle(value, "move text events toggle")?;
            }
        }
        "move_unknown_meta" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_unknown_meta_events =
                    parse_bool_toggle(value, "move unknown meta toggle")?;
            }
        }
        "move_channel_prefix" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_channel_prefix_events =
                    parse_bool_toggle(value, "move channel prefix toggle")?;
            }
        }
        "move_midi_port" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_midi_port_events = parse_bool_toggle(value, "move midi port toggle")?;
            }
        }
        "move_cc" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_control_change_events =
                    parse_bool_toggle(value, "move control change toggle")?;
            }
        }
        "move_program" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_program_change_events =
                    parse_bool_toggle(value, "move program change toggle")?;
            }
        }
        "move_pitch_bend" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_pitch_bend_events =
                    parse_bool_toggle(value, "move pitch bend toggle")?;
            }
        }
        "move_channel_pressure" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(config, "shared_metadata_track")
            {
                tool.move_channel_pressure_events =
                    parse_bool_toggle(value, "move channel pressure toggle")?;
            }
        }
        other => return Err(format!("unknown shared metadata control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_change_ppq_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ChangePpq(tool) = &config.tool {
        app.set_modify_change_ppq_text(tool.ppq.to_string().into());
    }
}

pub(super) fn update_change_ppq_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "ppq" => {
            if let MidiModifierTool::ChangePpq(tool) = ensure_tool_for_pass(config, "change_ppq") {
                tool.ppq = parse_value(value, "target PPQ")?;
            }
        }
        other => return Err(format!("unknown change ppq control: {other}")),
    }
    Ok(())
}

pub(super) fn sync_extract_track_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ExtractTrack(tool) = &config.tool {
        app.set_modify_extract_track_index_text(tool.track_index.to_string().into());
    }
}

pub(super) fn update_extract_track_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "track_index" => {
            if let MidiModifierTool::ExtractTrack(tool) =
                ensure_tool_for_pass(config, "extract_track")
            {
                tool.track_index = parse_value(value, "track index")?;
            }
        }
        other => return Err(format!("unknown extract track control: {other}")),
    }
    Ok(())
}
