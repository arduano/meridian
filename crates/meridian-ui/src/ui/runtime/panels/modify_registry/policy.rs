//! Modify-panel policy table and default tool builders.
//!
//! This file maps pass keys to human-facing labels, policy hints, and the
//! default tool constructor for each modify preset.

use super::super::modify_pass_controls::*;
use super::*;

pub(crate) struct ModifyPassPolicy {
    pub key: &'static str,
    pub field_prefix: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
    pub default_tool: fn() -> MidiModifierTool,
    pub sync_controls: fn(&App, &MidiFileProcessingConfig),
    pub update_control: fn(&App, &mut MidiFileProcessingConfig, &str, &str) -> Result<(), String>,
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

pub(crate) const MODIFY_PASS_POLICIES: &[ModifyPassPolicy] = &[
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

pub(crate) fn modify_pass_policy(pass_key: &str) -> Option<&'static ModifyPassPolicy> {
    MODIFY_PASS_POLICIES
        .iter()
        .find(|policy| policy.key == pass_key)
}

pub(crate) fn modify_pass_policy_for_field_prefix(
    field_prefix: &str,
) -> Option<&'static ModifyPassPolicy> {
    MODIFY_PASS_POLICIES
        .iter()
        .find(|policy| policy.field_prefix == field_prefix)
}
