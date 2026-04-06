use std::path::Path;

use crate::error::MeridianError;

use super::tools::MidiModifierTool;

pub mod change_ppq;
pub mod channel_remap;
mod common;
pub mod control_change;
pub mod extract_track;
pub mod key_map;
pub mod meta_text;
pub mod pitch_bend;
pub mod program;
pub mod sysex;
pub mod tempo_map;
pub mod velocity_map;

pub use change_ppq::ChangePpqTool;
pub use channel_remap::{ChannelMapEntry, ChannelRemapTool};
pub use control_change::{
    ControlChangeTool, ControlValue, ControllerMapEntry, ControllerScaleEntry,
};
pub use extract_track::ExtractTrackTool;
pub use key_map::{KeyMapEntry, KeyMapTool, KeyRange};
pub use meta_text::{MetaTextTool, TextKind};
pub use pitch_bend::PitchBendTool;
pub use program::{ChannelProgram, ProgramTool};
pub use sysex::SysexTool;
pub use tempo_map::{TempoMapTool, TempoPoint};
pub use velocity_map::{VelocityMapTool, VelocityPoint};

pub fn apply_modifier_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &MidiModifierTool,
) -> Result<(), MeridianError> {
    match tool {
        MidiModifierTool::ChannelRemap(tool) => {
            channel_remap::apply_channel_remap_tool_to_file(input, output, tool)
        }
        MidiModifierTool::ChangePpq(tool) => {
            change_ppq::apply_change_ppq_tool_to_file(input, output, tool)
        }
        MidiModifierTool::ControlChange(tool) => {
            control_change::apply_control_change_tool_to_file(input, output, tool)
        }
        MidiModifierTool::ExtractTrack(tool) => {
            extract_track::apply_extract_track_tool_to_file(input, output, tool)
        }
        MidiModifierTool::KeyMap(tool) => key_map::apply_key_map_tool_to_file(input, output, tool),
        MidiModifierTool::MetaText(tool) => {
            meta_text::apply_meta_text_tool_to_file(input, output, tool)
        }
        MidiModifierTool::PitchBend(tool) => {
            pitch_bend::apply_pitch_bend_tool_to_file(input, output, tool)
        }
        MidiModifierTool::Program(tool) => program::apply_program_tool_to_file(input, output, tool),
        MidiModifierTool::Sysex(tool) => sysex::apply_sysex_tool_to_file(input, output, tool),
        MidiModifierTool::TempoMap(tool) => {
            tempo_map::apply_tempo_map_tool_to_file(input, output, tool)
        }
        MidiModifierTool::VelocityMap(tool) => {
            velocity_map::apply_velocity_map_tool_to_file(input, output, tool)
        }
        other => Err(MeridianError::Unsupported(format!(
            "modifier tool not implemented in new modifier pipeline: {}",
            tool_name(other)
        ))),
    }
}

fn tool_name(tool: &MidiModifierTool) -> &'static str {
    match tool {
        MidiModifierTool::RangeSelect(_) => "range_select",
        MidiModifierTool::TempoMap(_) => "tempo_map",
        MidiModifierTool::TimeWarp(_) => "time_warp",
        MidiModifierTool::ChannelRemap(_) => "channel_remap",
        MidiModifierTool::ChangePpq(_) => "change_ppq",
        MidiModifierTool::TrackRoute(_) => "track_route",
        MidiModifierTool::Program(_) => "program",
        MidiModifierTool::ControlChange(_) => "control_change",
        MidiModifierTool::PitchBend(_) => "pitch_bend",
        MidiModifierTool::VelocityMap(_) => "velocity_map",
        MidiModifierTool::ExtractTrack(_) => "extract_track",
        MidiModifierTool::NoteLength(_) => "note_length",
        MidiModifierTool::OverlapRepair(_) => "overlap_repair",
        MidiModifierTool::Quantize(_) => "quantize",
        MidiModifierTool::Humanize(_) => "humanize",
        MidiModifierTool::KeyMap(_) => "key_map",
        MidiModifierTool::Dedupe(_) => "dedupe",
        MidiModifierTool::MetaText(_) => "meta_text",
        MidiModifierTool::Sysex(_) => "sysex",
        MidiModifierTool::MergeBalance(_) => "merge_balance",
        MidiModifierTool::SharedMetadataTrack(_) => "shared_metadata_track",
        MidiModifierTool::AnalysisGuard(_) => "analysis_guard",
    }
}

#[cfg(test)]
mod tests {
    use super::apply_modifier_tool_to_file;
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool, RangeSelectTool,
            test_support::{TestDir, note_on, write_toolkit_midi},
        },
    };

    #[test]
    fn unsupported_tools_fail_explicitly() {
        let dir = TestDir::new("modifier-unsupported");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool::default()),
        )
        .expect_err("unsupported tool should fail");

        assert!(matches!(error, MeridianError::Unsupported(_)));
        assert!(!output.exists());
    }
}
