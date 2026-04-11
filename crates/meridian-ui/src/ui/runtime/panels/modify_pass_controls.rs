use super::*;

pub(super) fn reset_modify_pass_control_fields(app: &App) {
    app.set_modify_range_track_select_text("".into());
    app.set_modify_range_start_ticks_text("".into());
    app.set_modify_range_end_ticks_text("".into());
    app.set_modify_range_offset_ticks_text("".into());
    app.set_modify_range_preserve_system_text("off".into());
    app.set_modify_range_edge_behavior_text("trim".into());
    app.set_modify_tempo_mode_text("scale_bpm".into());
    app.set_modify_tempo_flatten_tempo_text("500000".into());
    app.set_modify_tempo_scale_factor_text("1.0".into());
    app.set_modify_tempo_replace_points_text("".into());
    app.set_modify_time_warp_points_text("".into());
    app.set_modify_channel_remap_mappings_text("".into());
    app.set_modify_track_route_mode_text("collapse_all".into());
    app.set_modify_track_route_mappings_text("".into());
    app.set_modify_program_force_program_text("".into());
    app.set_modify_program_strip_changes_text("off".into());
    app.set_modify_program_startup_programs_text("".into());
    app.set_modify_control_strip_controllers_text("".into());
    app.set_modify_control_remap_controllers_text("".into());
    app.set_modify_control_scale_controllers_text("".into());
    app.set_modify_control_inject_start_text("".into());
    app.set_modify_pitch_strip_text("off".into());
    app.set_modify_pitch_scale_text("1.0".into());
    app.set_modify_pitch_offset_text("0".into());
    app.set_modify_pitch_min_bend_text("-8192".into());
    app.set_modify_pitch_max_bend_text("8191".into());
    app.set_modify_velocity_mode_text("scale".into());
    app.set_modify_velocity_scale_text("1.0".into());
    app.set_modify_velocity_gamma_text("1.0".into());
    app.set_modify_velocity_points_text("".into());
    app.set_modify_note_length_min_ticks_text("".into());
    app.set_modify_note_length_max_ticks_text("".into());
    app.set_modify_note_length_scale_text("1.0".into());
    app.set_modify_note_length_fixed_ticks_text("".into());
    app.set_modify_quantize_grid_ticks_text("0.25".into());
    app.set_modify_quantize_mode_text("note_start_only".into());
    app.set_modify_humanize_start_jitter_text("0".into());
    app.set_modify_humanize_length_jitter_text("0".into());
    app.set_modify_humanize_velocity_jitter_text("5".into());
    app.set_modify_humanize_seed_text("1".into());
    app.set_modify_humanize_collision_mode_text("stable".into());
    app.set_modify_key_map_mappings_text("".into());
    app.set_modify_key_map_fold_range_text("".into());
    app.set_modify_key_map_drop_unmapped_text("off".into());
    app.set_modify_meta_keep_kinds_text("".into());
    app.set_modify_sysex_strip_all_text("off".into());
    app.set_modify_sysex_prepend_text("".into());
    app.set_modify_change_ppq_text("480".into());
    app.set_modify_extract_track_index_text("0".into());
    app.set_modify_shared_dest_mode_text("create_new".into());
    app.set_modify_shared_dest_track_text("0".into());
    app.set_modify_shared_strip_redundant_text("off".into());
    app.set_modify_shared_move_tempo_text("on".into());
    app.set_modify_shared_move_time_sig_text("on".into());
    app.set_modify_shared_move_key_sig_text("on".into());
    app.set_modify_shared_move_text_text("off".into());
    app.set_modify_shared_move_unknown_meta_text("off".into());
    app.set_modify_shared_move_channel_prefix_text("off".into());
    app.set_modify_shared_move_midi_port_text("off".into());
    app.set_modify_shared_move_cc_text("off".into());
    app.set_modify_shared_move_program_text("off".into());
    app.set_modify_shared_move_pitch_bend_text("off".into());
    app.set_modify_shared_move_channel_pressure_text("off".into());
}

pub(super) fn sync_modify_pass_controls(app: &App, config: &MidiFileProcessingConfig) {
    reset_modify_pass_control_fields(app);

    match &config.tool {
        MidiModifierTool::RangeSelect(tool) => {
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
            app.set_modify_range_preserve_system_text(
                toggle_text(tool.preserve_system_events).into(),
            );
            app.set_modify_range_edge_behavior_text(format_serde_enum(&tool.edge_behavior).into());
        }
        MidiModifierTool::TempoMap(tool) => match tool {
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
        },
        MidiModifierTool::TimeWarp(tool) => {
            let ppq = get_ppq(app);
            app.set_modify_time_warp_points_text(
                format_time_warp_points_notes(&tool.points, ppq).into(),
            );
        }
        MidiModifierTool::ChannelRemap(tool) => {
            app.set_modify_channel_remap_mappings_text(
                format_channel_mappings(&tool.mappings).into(),
            );
        }
        MidiModifierTool::TrackRoute(tool) => match tool {
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
        },
        MidiModifierTool::Program(tool) => {
            app.set_modify_program_force_program_text(option_text(tool.force_program).into());
            app.set_modify_program_strip_changes_text(
                toggle_text(tool.strip_program_changes).into(),
            );
            app.set_modify_program_startup_programs_text(
                format_channel_programs(&tool.startup_programs).into(),
            );
        }
        MidiModifierTool::ControlChange(tool) => {
            app.set_modify_control_strip_controllers_text(
                format_u8_list(&tool.strip_controllers).into(),
            );
            app.set_modify_control_remap_controllers_text(
                format_controller_mappings(&tool.remap_controllers).into(),
            );
            app.set_modify_control_scale_controllers_text(
                format_controller_scales(&tool.scale_controllers).into(),
            );
            app.set_modify_control_inject_start_text(
                format_control_values(&tool.inject_start).into(),
            );
        }
        MidiModifierTool::PitchBend(tool) => {
            app.set_modify_pitch_strip_text(toggle_text(tool.strip).into());
            app.set_modify_pitch_scale_text(tool.scale.to_string().into());
            app.set_modify_pitch_offset_text(tool.offset.to_string().into());
            app.set_modify_pitch_min_bend_text(tool.min_bend.to_string().into());
            app.set_modify_pitch_max_bend_text(tool.max_bend.to_string().into());
        }
        MidiModifierTool::VelocityMap(tool) => match tool {
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
        },
        MidiModifierTool::ChangePpq(tool) => {
            app.set_modify_change_ppq_text(tool.ppq.to_string().into());
        }
        MidiModifierTool::ExtractTrack(tool) => {
            app.set_modify_extract_track_index_text(tool.track_index.to_string().into());
        }
        MidiModifierTool::NoteLength(tool) => {
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
        MidiModifierTool::Quantize(tool) => {
            let ppq = get_ppq(app);
            app.set_modify_quantize_grid_ticks_text(
                format_notes(tool.rounding_ticks as u64, ppq).into(),
            );
            app.set_modify_quantize_mode_text(format_serde_enum(&tool.mode).into());
        }
        MidiModifierTool::Humanize(tool) => {
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
        MidiModifierTool::KeyMap(tool) => {
            app.set_modify_key_map_mappings_text(format_key_mappings(&tool.mappings).into());
            app.set_modify_key_map_fold_range_text(format_key_range(&tool.fold_to_range).into());
            app.set_modify_key_map_drop_unmapped_text(toggle_text(tool.drop_unmapped).into());
        }
        MidiModifierTool::MetaText(tool) => {
            app.set_modify_meta_keep_kinds_text(format_text_kinds(&tool.keep_kinds).into());
        }
        MidiModifierTool::Sysex(tool) => {
            app.set_modify_sysex_strip_all_text(toggle_text(tool.strip_all).into());
            app.set_modify_sysex_prepend_text(format_sysex_messages(&tool.prepend).into());
        }
        MidiModifierTool::SharedMetadataTrack(tool) => {
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
            app.set_modify_shared_move_midi_port_text(
                toggle_text(tool.move_midi_port_events).into(),
            );
            app.set_modify_shared_move_cc_text(toggle_text(tool.move_control_change_events).into());
            app.set_modify_shared_move_program_text(
                toggle_text(tool.move_program_change_events).into(),
            );
            app.set_modify_shared_move_pitch_bend_text(
                toggle_text(tool.move_pitch_bend_events).into(),
            );
            app.set_modify_shared_move_channel_pressure_text(
                toggle_text(tool.move_channel_pressure_events).into(),
            );
        }
    }
}

pub(super) fn update_modify_control(app: &App, key: &str, value: &str) -> Result<(), String> {
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
                    points: Vec::new(),
                    destination: TempoMapDestination::InjectIntoFirstTrack,
                }),
                other => return Err(format!("invalid tempo mode: {other}")),
            };
            config.tool = tool;
        }
        "tempo.flatten_tempo" => {
            config.tool = MidiModifierTool::TempoMap(TempoMapTool::Flatten {
                tempo: parse_value(value, "flatten tempo")?,
            });
        }
        "tempo.scale_factor" => {
            config.tool = MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm {
                factor: parse_value(value, "tempo scale factor")?,
            });
        }
        "tempo.replace_points" => {
            let ppq = get_ppq(app);
            config.tool = MidiModifierTool::TempoMap(TempoMapTool::Replace {
                points: parse_tempo_points_notes(value, ppq)?,
                destination: TempoMapDestination::InjectIntoFirstTrack,
            });
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
            let tool = match value.trim() {
                "collapse_all" => MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll),
                "split_by_channel" => MidiModifierTool::TrackRoute(TrackRouteTool::SplitByChannel),
                "map" => MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                    mappings: Vec::new(),
                }),
                other => return Err(format!("invalid track-route mode: {other}")),
            };
            config.tool = tool;
        }
        "track_route.mappings" => {
            config.tool = MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                mappings: parse_track_mappings(value)?,
            });
        }
        "program.force_program" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.force_program = parse_optional_value(value, "force program")?;
            }
        }
        "program.strip_program_changes" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.strip_program_changes = parse_bool_toggle(value, "strip program changes")?;
            }
        }
        "program.startup_programs" => {
            if let MidiModifierTool::Program(tool) = ensure_tool_for_pass(&mut config, "program") {
                tool.startup_programs = parse_channel_programs(value)?;
            }
        }
        "control_change.strip_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.strip_controllers = parse_u8_list(value, "controller number")?;
            }
        }
        "control_change.remap_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.remap_controllers = parse_controller_mappings(value)?;
            }
        }
        "control_change.scale_controllers" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.scale_controllers = parse_controller_scales(value)?;
            }
        }
        "control_change.inject_start" => {
            if let MidiModifierTool::ControlChange(tool) =
                ensure_tool_for_pass(&mut config, "control_change")
            {
                tool.inject_start = parse_control_values(value)?;
            }
        }
        "pitch_bend.strip" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.strip = parse_bool_toggle(value, "pitch-bend strip toggle")?;
            }
        }
        "pitch_bend.scale" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.scale = parse_value(value, "pitch-bend scale")?;
            }
        }
        "pitch_bend.offset" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.offset = parse_value(value, "pitch-bend offset")?;
            }
        }
        "pitch_bend.min_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.min_bend = parse_value(value, "minimum bend")?;
            }
        }
        "pitch_bend.max_bend" => {
            if let MidiModifierTool::PitchBend(tool) =
                ensure_tool_for_pass(&mut config, "pitch_bend")
            {
                tool.max_bend = parse_value(value, "maximum bend")?;
            }
        }
        "velocity_map.mode" => {
            let tool = match value.trim() {
                "scale" => MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 1.0 }),
                "gamma" => MidiModifierTool::VelocityMap(VelocityMapTool::Gamma { gamma: 1.0 }),
                "polyline" => {
                    MidiModifierTool::VelocityMap(VelocityMapTool::Polyline { points: Vec::new() })
                }
                other => return Err(format!("invalid velocity-map mode: {other}")),
            };
            config.tool = tool;
        }
        "velocity_map.scale" => {
            config.tool = MidiModifierTool::VelocityMap(VelocityMapTool::Scale {
                scale: parse_value(value, "velocity scale")?,
            });
        }
        "velocity_map.gamma" => {
            config.tool = MidiModifierTool::VelocityMap(VelocityMapTool::Gamma {
                gamma: parse_value(value, "velocity gamma")?,
            });
        }
        "velocity_map.points" => {
            config.tool = MidiModifierTool::VelocityMap(VelocityMapTool::Polyline {
                points: parse_velocity_points(value)?,
            });
        }
        "note_length.min_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.min_ticks = parse_optional_notes(value, ppq, "minimum note length")?;
            }
        }
        "note_length.max_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.max_ticks = parse_optional_notes(value, ppq, "maximum note length")?;
            }
        }
        "note_length.scale" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                tool.scale = parse_optional_value(value, "note-length scale")?;
            }
        }
        "note_length.fixed_ticks" => {
            if let MidiModifierTool::NoteLength(tool) =
                ensure_tool_for_pass(&mut config, "note_length")
            {
                let ppq = get_ppq(app);
                tool.fixed_ticks = parse_optional_notes(value, ppq, "fixed note length")?;
            }
        }
        "quantize.grid_ticks" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                let ppq = get_ppq(app);
                tool.rounding_ticks = parse_notes(value, ppq, "quantize grid")?;
            }
        }
        "quantize.mode" => {
            if let MidiModifierTool::Quantize(tool) = ensure_tool_for_pass(&mut config, "quantize")
            {
                tool.mode = parse_serde_enum(value, "quantize mode")?;
            }
        }
        "humanize.start_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                let ppq = get_ppq(app);
                tool.start_jitter = parse_notes(value, ppq, "humanize start jitter")? as i64;
            }
        }
        "humanize.length_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                let ppq = get_ppq(app);
                tool.length_jitter = parse_notes(value, ppq, "humanize length jitter")? as i64;
            }
        }
        "humanize.velocity_jitter" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.velocity_jitter = parse_value(value, "humanize velocity jitter")?;
            }
        }
        "humanize.seed" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
            {
                tool.seed = parse_value(value, "humanize seed")?;
            }
        }
        "humanize.collision_mode" => {
            if let MidiModifierTool::Humanize(tool) = ensure_tool_for_pass(&mut config, "humanize")
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
