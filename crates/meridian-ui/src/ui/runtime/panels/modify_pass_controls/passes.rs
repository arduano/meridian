//! Per-pass modify control implementations.
//!
//! The policy registry points at the functions in this file.

use super::*;

pub(crate) fn sync_range_select_controls(app: &App, config: &MidiFileProcessingConfig) {
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

pub(crate) fn update_range_select_control(
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

pub(crate) fn sync_tempo_map_controls(app: &App, config: &MidiFileProcessingConfig) {
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
                app.set_modify_tempo_replace_destination_text(
                    format_serde_enum(destination).into(),
                );
            }
        }
    }
}

pub(crate) fn update_tempo_map_control(
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

pub(crate) fn sync_time_warp_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::TimeWarp(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_time_warp_points_text(
            format_time_warp_points_notes(&tool.points, ppq).into(),
        );
    }
}

pub(crate) fn update_time_warp_control(
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

pub(crate) fn sync_channel_remap_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ChannelRemap(tool) = &config.tool {
        app.set_modify_channel_remap_mappings_text(format_channel_mappings(&tool.mappings).into());
    }
}

pub(crate) fn update_channel_remap_control(
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

pub(crate) fn sync_track_route_controls(app: &App, config: &MidiFileProcessingConfig) {
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

pub(crate) fn update_track_route_control(
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

pub(crate) fn sync_program_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Program(tool) = &config.tool {
        app.set_modify_program_force_program_text(option_text(tool.force_program).into());
        app.set_modify_program_strip_changes_text(toggle_text(tool.strip_program_changes).into());
        app.set_modify_program_startup_programs_text(
            format_channel_programs(&tool.startup_programs).into(),
        );
    }
}

pub(crate) fn update_program_control(
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

pub(crate) fn sync_control_change_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ControlChange(tool) = &config.tool {
        app.set_modify_control_strip_controllers_text(
            format_u8_list(&tool.strip_controllers).into(),
        );
        app.set_modify_control_remap_controllers_text(
            format_controller_mappings(&tool.remap_controllers).into(),
        );
        app.set_modify_control_scale_controllers_text(
            format_controller_scales(&tool.scale_controllers).into(),
        );
        app.set_modify_control_inject_start_text(format_control_values(&tool.inject_start).into());
    }
}

pub(crate) fn update_control_change_control(
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

pub(crate) fn sync_pitch_bend_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::PitchBend(tool) = &config.tool {
        app.set_modify_pitch_strip_text(toggle_text(tool.strip).into());
        app.set_modify_pitch_scale_text(tool.scale.to_string().into());
        app.set_modify_pitch_offset_text(tool.offset.to_string().into());
        app.set_modify_pitch_min_bend_text(tool.min_bend.to_string().into());
        app.set_modify_pitch_max_bend_text(tool.max_bend.to_string().into());
    }
}

pub(crate) fn update_pitch_bend_control(
    _app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "strip" => {
            if let MidiModifierTool::PitchBend(tool) = ensure_tool_for_pass(config, "pitch_bend") {
                tool.strip = parse_bool_toggle(value, "pitch bend strip toggle")?;
            }
        }
        "scale" => {
            if let MidiModifierTool::PitchBend(tool) = ensure_tool_for_pass(config, "pitch_bend") {
                tool.scale = parse_value(value, "pitch bend scale")?;
            }
        }
        "offset" => {
            if let MidiModifierTool::PitchBend(tool) = ensure_tool_for_pass(config, "pitch_bend") {
                tool.offset = parse_value(value, "pitch bend offset")?;
            }
        }
        "min_bend" => {
            if let MidiModifierTool::PitchBend(tool) = ensure_tool_for_pass(config, "pitch_bend") {
                tool.min_bend = parse_value(value, "pitch bend min")?;
            }
        }
        "max_bend" => {
            if let MidiModifierTool::PitchBend(tool) = ensure_tool_for_pass(config, "pitch_bend") {
                tool.max_bend = parse_value(value, "pitch bend max")?;
            }
        }
        other => return Err(format!("unknown pitch bend control: {other}")),
    }
    Ok(())
}

pub(crate) fn sync_velocity_map_controls(app: &App, config: &MidiFileProcessingConfig) {
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

pub(crate) fn update_velocity_map_control(
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
                "polyline" => {
                    MidiModifierTool::VelocityMap(VelocityMapTool::Polyline { points: vec![] })
                }
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

pub(crate) fn sync_note_length_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::NoteLength(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_note_length_min_ticks_text(format_option_notes(tool.min_ticks, ppq).into());
        app.set_modify_note_length_max_ticks_text(format_option_notes(tool.max_ticks, ppq).into());
        app.set_modify_note_length_scale_text(option_text(tool.scale).into());
        app.set_modify_note_length_fixed_ticks_text(
            format_option_notes(tool.fixed_ticks, ppq).into(),
        );
    }
}

pub(crate) fn update_note_length_control(
    app: &App,
    config: &mut MidiFileProcessingConfig,
    field: &str,
    value: &str,
) -> Result<(), String> {
    match field {
        "min_ticks" => {
            if let MidiModifierTool::NoteLength(tool) = ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.min_ticks = parse_optional_notes(value, ppq, "note length min")?;
            }
        }
        "max_ticks" => {
            if let MidiModifierTool::NoteLength(tool) = ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.max_ticks = parse_optional_notes(value, ppq, "note length max")?;
            }
        }
        "scale" => {
            if let MidiModifierTool::NoteLength(tool) = ensure_tool_for_pass(config, "note_length")
            {
                tool.scale = parse_optional_value(value, "note length scale")?;
            }
        }
        "fixed_ticks" => {
            if let MidiModifierTool::NoteLength(tool) = ensure_tool_for_pass(config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.fixed_ticks = parse_optional_notes(value, ppq, "note length fixed")?;
            }
        }
        other => return Err(format!("unknown note length control: {other}")),
    }
    Ok(())
}

pub(crate) fn sync_quantize_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Quantize(tool) = &config.tool {
        let ppq = get_ppq(app);
        app.set_modify_quantize_grid_ticks_text(
            format_notes(tool.rounding_ticks, ppq).into(),
        );
        app.set_modify_quantize_mode_text(format_serde_enum(&tool.mode).into());
    }
}

pub(crate) fn update_quantize_control(
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

pub(crate) fn sync_humanize_controls(app: &App, config: &MidiFileProcessingConfig) {
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
        app.set_modify_humanize_collision_mode_text(format_serde_enum(&tool.collision_mode).into());
    }
}

pub(crate) fn update_humanize_control(
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

pub(crate) fn sync_key_map_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::KeyMap(tool) = &config.tool {
        app.set_modify_key_map_mappings_text(format_key_mappings(&tool.mappings).into());
        app.set_modify_key_map_fold_range_text(format_key_range(&tool.fold_to_range).into());
        app.set_modify_key_map_drop_unmapped_text(toggle_text(tool.drop_unmapped).into());
    }
}

pub(crate) fn update_key_map_control(
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

pub(crate) fn sync_meta_text_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::MetaText(tool) = &config.tool {
        app.set_modify_meta_keep_kinds_text(format_text_kinds(&tool.keep_kinds).into());
    }
}

pub(crate) fn update_meta_text_control(
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

pub(crate) fn sync_sysex_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::Sysex(tool) = &config.tool {
        app.set_modify_sysex_strip_all_text(toggle_text(tool.strip_all).into());
        app.set_modify_sysex_prepend_text(format_sysex_messages(&tool.prepend).into());
    }
}

pub(crate) fn update_sysex_control(
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

pub(crate) fn sync_shared_metadata_track_controls(app: &App, config: &MidiFileProcessingConfig) {
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
        app.set_modify_shared_strip_redundant_text(toggle_text(tool.strip_redundant_events).into());
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
        app.set_modify_shared_move_program_text(
            toggle_text(tool.move_program_change_events).into(),
        );
        app.set_modify_shared_move_pitch_bend_text(toggle_text(tool.move_pitch_bend_events).into());
        app.set_modify_shared_move_channel_pressure_text(
            toggle_text(tool.move_channel_pressure_events).into(),
        );
    }
}

pub(crate) fn update_shared_metadata_track_control(
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
                    "insert_into" => SharedMetadataTrackDestination::InsertInto { track_index: 0 },
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
                tool.strip_redundant_events = parse_bool_toggle(value, "strip redundant toggle")?;
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
                tool.move_key_signatures = parse_bool_toggle(value, "move key signatures toggle")?;
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
                tool.move_pitch_bend_events = parse_bool_toggle(value, "move pitch bend toggle")?;
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

pub(crate) fn sync_change_ppq_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ChangePpq(tool) = &config.tool {
        app.set_modify_change_ppq_text(tool.ppq.to_string().into());
    }
}

pub(crate) fn update_change_ppq_control(
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

pub(crate) fn sync_extract_track_controls(app: &App, config: &MidiFileProcessingConfig) {
    if let MidiModifierTool::ExtractTrack(tool) = &config.tool {
        app.set_modify_extract_track_index_text(tool.track_index.to_string().into());
    }
}

pub(crate) fn update_extract_track_control(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempo_map_destination_serde_labels_match_structured_ui_values() {
        assert_eq!(
            format_serde_enum(&TempoMapDestination::InjectIntoFirstTrack),
            "inject_into_first_track"
        );
        assert_eq!(
            format_serde_enum(&TempoMapDestination::CreateNewTempoTrack),
            "create_new_tempo_track"
        );
        assert_eq!(
            parse_serde_enum::<TempoMapDestination>("create_new_tempo_track", "tempo destination",)
                .unwrap(),
            TempoMapDestination::CreateNewTempoTrack
        );
    }
}
