use std::path::Path;

use crate::error::MeridianError;

use super::tools::MidiModifierTool;

pub mod channel_remap;
mod common;
pub mod key_map;
pub mod meta_text;

pub use channel_remap::{ChannelMapEntry, ChannelRemapTool};
pub use key_map::{KeyMapEntry, KeyMapTool, KeyRange};
pub use meta_text::{MetaTextTool, TextKind};

pub fn apply_modifier_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &MidiModifierTool,
) -> Result<(), MeridianError> {
    match tool {
        MidiModifierTool::ChannelRemap(tool) => {
            channel_remap::apply_channel_remap_tool_to_file(input, output, tool)
        }
        MidiModifierTool::KeyMap(tool) => key_map::apply_key_map_tool_to_file(input, output, tool),
        MidiModifierTool::MetaText(tool) => {
            meta_text::apply_meta_text_tool_to_file(input, output, tool)
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
        MidiModifierTool::TrackRoute(_) => "track_route",
        MidiModifierTool::Program(_) => "program",
        MidiModifierTool::ControlChange(_) => "control_change",
        MidiModifierTool::PitchBend(_) => "pitch_bend",
        MidiModifierTool::VelocityMap(_) => "velocity_map",
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
