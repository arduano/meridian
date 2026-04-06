use std::path::Path;

use crate::error::MeridianError;

use super::tools::MidiModifierTool;

mod common;
mod key_map;

pub fn apply_modifier_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &MidiModifierTool,
) -> Result<(), MeridianError> {
    match tool {
        MidiModifierTool::KeyMap(tool) => key_map::apply_key_map_tool_to_file(input, output, tool),
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
    use midi_toolkit::events::MIDIEvent;

    use super::apply_modifier_tool_to_file;
    use crate::{
        error::MeridianError,
        midi::{
            KeyMapEntry, KeyMapTool, KeyRange, MidiModifierTool, RangeSelectTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn key_map_pass_maps_folds_and_drops_keys() {
        let dir = TestDir::new("modifier-key-map");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(0, 0, 60, 100),
                note_on(0, 0, 85, 100),
                note_on(0, 0, 61, 100),
                note_off(10, 0, 60),
                note_off(0, 0, 85),
                note_off(0, 0, 61),
            ]],
        );

        apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::KeyMap(KeyMapTool {
                mappings: vec![KeyMapEntry { from: 60, to: 72 }],
                fold_to_range: Some(KeyRange { min: 70, max: 72 }),
                drop_unmapped: true,
            }),
        )
        .expect("key map should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let keys = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| event.event.key())
            .collect::<Vec<_>>();

        assert_eq!(parsed.midi().track_count(), 1);
        assert_eq!(parsed.midi().ppq(), 96);
        assert_eq!(keys, vec![72, 70, 70, 72, 70, 70]);
    }

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
