use std::path::Path;

use crate::error::MeridianError;

use super::{parsed::ParsedMidiFile, tools::MidiModifierTool};

pub mod change_ppq;
pub mod channel_remap;
mod common;
pub mod control_change;
pub mod extract_track;
pub mod humanize;
pub mod key_map;
pub mod meta_text;
pub mod note_length;
pub mod pitch_bend;
pub mod program;
pub mod quantize;
pub mod range_select;
pub mod shared_metadata_track;
pub mod sysex;
pub mod tempo_map;
pub mod time_warp;
pub mod track_route;
pub mod velocity_map;

pub use change_ppq::ChangePpqTool;
pub use channel_remap::{ChannelMapEntry, ChannelRemapTool};
pub use control_change::{
    ControlChangeTool, ControlValue, ControllerMapEntry, ControllerScaleEntry,
};
pub use extract_track::ExtractTrackTool;
pub use humanize::{HumanizeCollisionMode, HumanizeTool};
pub use key_map::{KeyMapEntry, KeyMapTool, KeyRange};
pub use meta_text::{MetaTextTool, TextKind};
pub use note_length::NoteLengthTool;
pub use pitch_bend::PitchBendTool;
pub use program::{ChannelProgram, ProgramTool};
pub use quantize::{QuantizeMode, QuantizeTool};
pub use range_select::{RangeEdgeBehavior, RangeSelectTool};
pub use shared_metadata_track::SharedMetadataTrackTool;
pub use sysex::SysexTool;
pub use tempo_map::{TempoMapTool, TempoPoint};
pub use time_warp::{TimeWarpPoint, TimeWarpTool};
pub use track_route::{TrackMapEntry, TrackRouteTool};
pub use velocity_map::{VelocityMapTool, VelocityPoint};

pub fn apply_modifier_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &MidiModifierTool,
) -> Result<(), MeridianError> {
    let parsed = common::load_parsed_midi(input)?;
    let mut on_progress = |_: u8, _: &str| {};
    let should_cancel = || false;
    apply_modifier_tool_to_parsed_file(output, tool, &parsed, &mut on_progress, &should_cancel)
}

pub fn apply_modifier_tool_to_parsed_file(
    output: &Path,
    tool: &MidiModifierTool,
    parsed: &ParsedMidiFile,
    on_progress: &mut dyn FnMut(u8, &str),
    should_cancel: &dyn Fn() -> bool,
) -> Result<(), MeridianError> {
    let mut progress = common::ToolProgress::new(on_progress, should_cancel);
    match tool {
        MidiModifierTool::ChannelRemap(tool) => {
            channel_remap::apply_channel_remap_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
        MidiModifierTool::ChangePpq(tool) => {
            change_ppq::apply_change_ppq_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::ControlChange(tool) => {
            control_change::apply_control_change_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
        MidiModifierTool::ExtractTrack(tool) => {
            extract_track::apply_extract_track_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
        MidiModifierTool::Humanize(tool) => {
            humanize::apply_humanize_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::KeyMap(tool) => {
            key_map::apply_key_map_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::MetaText(tool) => {
            meta_text::apply_meta_text_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::NoteLength(tool) => {
            note_length::apply_note_length_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::PitchBend(tool) => {
            pitch_bend::apply_pitch_bend_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::Program(tool) => {
            program::apply_program_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::Quantize(tool) => {
            quantize::apply_quantize_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::RangeSelect(tool) => {
            range_select::apply_range_select_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
        MidiModifierTool::SharedMetadataTrack(tool) => {
            shared_metadata_track::apply_shared_metadata_track_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
        MidiModifierTool::Sysex(tool) => {
            sysex::apply_sysex_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::TempoMap(tool) => {
            tempo_map::apply_tempo_map_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::TimeWarp(tool) => {
            time_warp::apply_time_warp_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::TrackRoute(tool) => {
            track_route::apply_track_route_tool_to_parsed_file(parsed, output, tool, &mut progress)
        }
        MidiModifierTool::VelocityMap(tool) => {
            velocity_map::apply_velocity_map_tool_to_parsed_file(
                parsed,
                output,
                tool,
                &mut progress,
            )
        }
    }
}
