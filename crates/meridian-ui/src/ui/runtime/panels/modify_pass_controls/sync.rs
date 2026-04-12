use super::*;

pub(in super::super) fn reset_modify_pass_control_fields(app: &App) {
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

pub(in super::super) fn sync_modify_pass_controls(app: &App, config: &MidiFileProcessingConfig) {
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
