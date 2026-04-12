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
    app.set_modify_tempo_replace_destination_text("inject_into_first_track".into());
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

    if let Some(policy) = modify_pass_policy(tool_key(&config.tool)) {
        (policy.sync_controls)(app, config);
    }
}
