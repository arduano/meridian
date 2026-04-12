use super::*;

pub(in super::super) fn update_modify_control(
    app: &App,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let mut config = fallback_modify_config(app);

    match key {
        "range.track_select" | "range.offset_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                if key == "range.track_select" {
                    tool.track_select = parse_optional_value(value, "range track")?;
                } else {
                    let ppq = get_ppq(app);
                    tool.offset_ticks = parse_optional_notes(value, ppq, "range offset")?;
                }
            }
        }
        "range.preserve_system_events" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.preserve_system_events = parse_bool_toggle(value, "preserve system events")?;
            }
        }
        "range.edge_behavior" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                tool.edge_behavior = parse_serde_enum(value, "range edge behavior")?;
            }
        }
        "range.start_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                let ppq = get_ppq(app);
                tool.start_ticks = parse_notes(value, ppq, "range start")?;
            }
        }
        "range.end_ticks" => {
            if let MidiModifierTool::RangeSelect(tool) =
                ensure_tool_for_pass(&mut config, "range_select")
            {
                let ppq = get_ppq(app);
                tool.end_ticks = parse_notes(value, ppq, "range end")?;
            }
        }
        "tempo.mode" => {
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
        "tempo.flatten_tempo" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Flatten { tempo }) =
                ensure_tool_for_pass(&mut config, "tempo_map")
            {
                *tempo = parse_value(value, "flatten tempo")?;
            }
        }
        "tempo.scale_factor" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm { factor }) =
                ensure_tool_for_pass(&mut config, "tempo_map")
            {
                *factor = parse_value(value, "scale factor")?;
            }
        }
        "tempo.replace_points" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Replace { points, .. }) =
                ensure_tool_for_pass(&mut config, "tempo_map")
            {
                let ppq = get_ppq(app);
                *points = parse_tempo_points_notes(value, ppq)?;
            }
        }
        "tempo.destination" => {
            if let MidiModifierTool::TempoMap(TempoMapTool::Replace { destination, .. }) =
                ensure_tool_for_pass(&mut config, "tempo_map")
            {
                *destination = parse_serde_enum(value, "tempo destination")?;
            }
        }
        "time_warp.points" => {
            if let MidiModifierTool::TimeWarp(tool) = ensure_tool_for_pass(&mut config, "time_warp")
            {
                let ppq = get_ppq(app);
                tool.points = parse_time_warp_points_notes(value, ppq)?;
            }
        }
        "channel_remap.mappings" => {
            if let MidiModifierTool::ChannelRemap(tool) =
                ensure_tool_for_pass(&mut config, "channel_remap")
            {
                tool.mappings = parse_channel_mappings(value)?;
            }
        }
        "track_route.mode" => {
            if let MidiModifierTool::TrackRoute(tool) = ensure_tool_for_pass(&mut config, "track_route")
            {
                *tool = match value.trim() {
                    "collapse_all" => TrackRouteTool::CollapseAll,
                    "split_by_channel" => TrackRouteTool::SplitByChannel,
                    "map" => TrackRouteTool::Map { mappings: vec![] },
                    other => return Err(format!("invalid track route mode: {other}")),
                };
            }
        }
        "track_route.mappings" => {
            if let MidiModifierTool::TrackRoute(TrackRouteTool::Map { mappings }) =
                ensure_tool_for_pass(&mut config, "track_route")
            {
                *mappings = parse_track_mappings(value)?;
            }
        }
        "program.force_program" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program")
            {
                tool.force_program = parse_optional_value(value, "program number")?;
            }
        }
        "program.strip_program_changes" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program")
            {
                tool.strip_program_changes = parse_bool_toggle(value, "program strip toggle")?;
            }
        }
        "program.startup_programs" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program")
            {
                tool.startup_programs = parse_channel_programs(value)?;
            }
        }
        "control.strip_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.strip_controllers = parse_u8_list(value, "strip controller")?;
            }
        }
        "control.remap_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.remap_controllers = parse_controller_mappings(value)?;
            }
        }
        "control.scale_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.scale_controllers = parse_controller_scales(value)?;
            }
        }
        "control.inject_start" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.inject_start = parse_control_values(value)?;
            }
        }
        "pitch.strip" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.strip = parse_bool_toggle(value, "pitch bend strip toggle")?;
            }
        }
        "pitch.scale" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.scale = parse_value(value, "pitch bend scale")?;
            }
        }
        "pitch.offset" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.offset = parse_value(value, "pitch bend offset")?;
            }
        }
        "pitch.min_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.min_bend = parse_value(value, "pitch bend min")?;
            }
        }
        "pitch.max_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.max_bend = parse_value(value, "pitch bend max")?;
            }
        }
        "velocity.mode" => {
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
        "velocity.scale" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale }) =
                ensure_tool_for_pass(&mut config, "velocity_map")
            {
                *scale = parse_value(value, "velocity scale")?;
            }
        }
        "velocity.gamma" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Gamma { gamma }) =
                ensure_tool_for_pass(&mut config, "velocity_map")
            {
                *gamma = parse_value(value, "velocity gamma")?;
            }
        }
        "velocity.points" => {
            if let MidiModifierTool::VelocityMap(VelocityMapTool::Polyline { points }) =
                ensure_tool_for_pass(&mut config, "velocity_map")
            {
                *points = parse_velocity_points(value)?;
            }
        }
        "note_length.min_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.min_ticks = parse_optional_notes(value, ppq, "note length min")?;
            }
        }
        "note_length.max_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.max_ticks = parse_optional_notes(value, ppq, "note length max")?;
            }
        }
        "note_length.scale" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.scale = parse_optional_value(value, "note length scale")?;
            }
        }
        "note_length.fixed_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.fixed_ticks = parse_optional_notes(value, ppq, "note length fixed")?;
            }
        }
        "quantize.grid_ticks" => {
            if let MidiModifierTool::Quantize(tool) =
                ensure_tool_for_pass(&mut config, "quantize")
            {
                let ppq = get_ppq(app);
                tool.rounding_ticks = parse_notes(value, ppq, "quantize grid")?;
            }
        }
        "quantize.mode" => {
            if let MidiModifierTool::Quantize(tool) =
                ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.mode = parse_serde_enum(value, "quantize mode")?;
            }
        }
        "humanize.start_jitter" => {
            if let MidiModifierTool::Humanize(tool) =
                ensure_tool_for_pass(&mut config, "humanize")
            {
                let ppq = get_ppq(app);
                tool.start_jitter = parse_notes(value, ppq, "humanize start jitter")? as i64;
            }
        }
        "humanize.length_jitter" => {
            if let MidiModifierTool::Humanize(tool) =
                ensure_tool_for_pass(&mut config, "humanize")
            {
                let ppq = get_ppq(app);
                tool.length_jitter = parse_notes(value, ppq, "humanize length jitter")? as i64;
            }
        }
        "humanize.velocity_jitter" => {
            if let MidiModifierTool::Humanize(tool) =
                ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.velocity_jitter = parse_value(value, "humanize velocity jitter")?;
            }
        }
        "humanize.seed" => {
            if let MidiModifierTool::Humanize(tool) =
                ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.seed = parse_value(value, "humanize seed")?;
            }
        }
        "humanize.collision_mode" => {
            if let MidiModifierTool::Humanize(tool) =
                ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.collision_mode = parse_serde_enum(value, "humanize collision mode")?;
            }
        }
        "key_map.mappings" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.mappings = parse_key_mappings(value)?;
            }
        }
        "key_map.fold_range" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.fold_to_range = parse_key_range(value)?;
            }
        }
        "key_map.drop_unmapped" => {
            if let MidiModifierTool::KeyMap(tool) = ensure_tool_for_pass(&mut config, "key_map") {
                tool.drop_unmapped = parse_bool_toggle(value, "drop-unmapped toggle")?;
            }
        }
        "meta.keep_kinds" => {
            if let MidiModifierTool::MetaText(tool) = ensure_tool_for_pass(&mut config, "meta_text")
            {
                tool.keep_kinds = parse_text_kinds(value)?;
            }
        }
        "sysex.strip_all" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(&mut config, "sysex") {
                tool.strip_all = parse_bool_toggle(value, "sysex strip-all toggle")?;
            }
        }
        "sysex.prepend" => {
            if let MidiModifierTool::Sysex(tool) = ensure_tool_for_pass(&mut config, "sysex") {
                tool.prepend = parse_sysex_messages(value)?;
            }
        }
        "change_ppq.ppq" => {
            if let MidiModifierTool::ChangePpq(tool) =
                ensure_tool_for_pass(&mut config, "change_ppq")
            {
                tool.ppq = parse_value(value, "target PPQ")?;
            }
        }
        "extract_track.track_index" => {
            if let MidiModifierTool::ExtractTrack(tool) =
                ensure_tool_for_pass(&mut config, "extract_track")
            {
                tool.track_index = parse_value(value, "track index")?;
            }
        }
        "shared_metadata.dest_mode" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.destination = match value.trim() {
                    "create_new" => SharedMetadataTrackDestination::CreateNew,
                    "insert_into" => SharedMetadataTrackDestination::InsertInto { track_index: 0 },
                    other => return Err(format!("invalid destination mode: {other}")),
                };
            }
        }
        "shared_metadata.dest_track" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                let idx = parse_value(value, "destination track index")?;
                tool.destination = SharedMetadataTrackDestination::InsertInto { track_index: idx };
            }
        }
        "shared_metadata.strip_redundant" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.strip_redundant_events = parse_bool_toggle(value, "strip redundant toggle")?;
            }
        }
        "shared_metadata.move_tempo" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_tempo_events = parse_bool_toggle(value, "move tempo toggle")?;
            }
        }
        "shared_metadata.move_time_sig" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_time_signatures =
                    parse_bool_toggle(value, "move time signatures toggle")?;
            }
        }
        "shared_metadata.move_key_sig" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_key_signatures = parse_bool_toggle(value, "move key signatures toggle")?;
            }
        }
        "shared_metadata.move_text" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_text_events = parse_bool_toggle(value, "move text events toggle")?;
            }
        }
        "shared_metadata.move_unknown_meta" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_unknown_meta_events =
                    parse_bool_toggle(value, "move unknown meta toggle")?;
            }
        }
        "shared_metadata.move_channel_prefix" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_channel_prefix_events =
                    parse_bool_toggle(value, "move channel prefix toggle")?;
            }
        }
        "shared_metadata.move_midi_port" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_midi_port_events = parse_bool_toggle(value, "move midi port toggle")?;
            }
        }
        "shared_metadata.move_cc" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_control_change_events =
                    parse_bool_toggle(value, "move control change toggle")?;
            }
        }
        "shared_metadata.move_program" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_program_change_events =
                    parse_bool_toggle(value, "move program change toggle")?;
            }
        }
        "shared_metadata.move_pitch_bend" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_pitch_bend_events = parse_bool_toggle(value, "move pitch bend toggle")?;
            }
        }
        "shared_metadata.move_channel_pressure" => {
            if let MidiModifierTool::SharedMetadataTrack(tool) =
                ensure_tool_for_pass(&mut config, "shared_metadata_track")
            {
                tool.move_channel_pressure_events =
                    parse_bool_toggle(value, "move channel pressure toggle")?;
            }
        }
        other => return Err(format!("unknown modify control: {other}")),
    }

    set_modify_config(app, &config);
    Ok(())
}
