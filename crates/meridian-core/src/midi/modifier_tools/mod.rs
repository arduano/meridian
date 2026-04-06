use std::path::Path;

use crate::error::MeridianError;

use super::tools::MidiModifierTool;

mod channel_remap;
mod common;
mod key_map;
mod meta_text;

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
    use midi_toolkit::events::MIDIEvent;
    use midi_toolkit::events::{Event, TextEvent, TextEventKind};
    use midi_toolkit::sequence::event::Delta;

    use super::apply_modifier_tool_to_file;
    use crate::{
        error::MeridianError,
        midi::{
            ChannelMapEntry, ChannelRemapTool, KeyMapEntry, KeyMapTool, KeyRange, MetaTextTool,
            MidiModifierTool, RangeSelectTool, TextKind,
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

    #[test]
    fn channel_remap_pass_updates_channel_events() {
        let dir = TestDir::new("modifier-channel-remap");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_note_on_event(0, 1, 60, 100),
                Event::new_delta_control_change_event(0, 1, 64, 127),
                Event::new_delta_program_change_event(0, 1, 10),
                note_off(24, 1, 60),
            ]],
        );

        apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChannelRemap(ChannelRemapTool {
                mappings: vec![ChannelMapEntry { from: 1, to: 9 }],
            }),
        )
        .expect("channel remap should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let channels = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| event.event.channel())
            .collect::<Vec<_>>();

        assert_eq!(channels, vec![9, 9, 9, 9]);
    }

    #[test]
    fn channel_remap_rejects_invalid_channel_maps() {
        let dir = TestDir::new("modifier-channel-remap-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChannelRemap(ChannelRemapTool {
                mappings: vec![
                    ChannelMapEntry { from: 2, to: 9 },
                    ChannelMapEntry { from: 2, to: 10 },
                ],
            }),
        )
        .expect_err("duplicate channel mappings should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }

    #[test]
    fn meta_text_pass_filters_track_text_events() {
        let dir = TestDir::new("modifier-meta-text");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Delta::new(
                    0,
                    Event::Text(Box::new(TextEvent {
                        kind: TextEventKind::TrackName,
                        bytes: b"Piano".to_vec(),
                    })),
                ),
                Delta::new(
                    12,
                    Event::Text(Box::new(TextEvent {
                        kind: TextEventKind::Marker,
                        bytes: b"Verse".to_vec(),
                    })),
                ),
                note_on(24, 0, 60, 100),
                note_off(24, 0, 60),
            ]],
        );

        apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::MetaText(MetaTextTool {
                keep_kinds: vec![TextKind::TrackName],
            }),
        )
        .expect("meta text should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::Text(text),
                },
                Delta {
                    delta: 36,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 24,
                    event: Event::NoteOff(_),
                }
            ] if text.kind == TextEventKind::TrackName && text.bytes == b"Piano"
        ));
    }
}
