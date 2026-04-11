use std::path::Path;

use midi_toolkit::{events::Event, prelude::EventSequenceExt, sequence::event::Delta};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::MeridianError,
    midi::{
        modifier_tools::common::{
            ToolProgress, finish_midi_writer, open_midi_writer, track_events,
            write_try_track_events,
        },
        parsed::ParsedMidiFile,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ExtractTrackTool {
    pub track_index: usize,
}

pub(super) fn apply_extract_track_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &ExtractTrackTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = format!("Extracting track {}", tool.track_index);
    progress.report(0, &label)?;
    validate_extract_track_tool(&parsed, tool)?;

    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let iter = extracted_track_events(&parsed, tool.track_index as u32)
        .expect("track iteration should exist for a validated track index");
    write_try_track_events(&writer, iter)?;
    progress.report(100, &label)?;
    finish_midi_writer(writer)
}

fn extracted_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(map_non_track_start_event))
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn validate_extract_track_tool(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &ExtractTrackTool,
) -> Result<(), MeridianError> {
    if tool.track_index < parsed.midi().track_count() {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "extract_track track_index {} is out of range for {} tracks",
            tool.track_index,
            parsed.midi().track_count()
        )))
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::ExtractTrackTool;
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn extract_track_writes_only_the_selected_track() {
        let dir = TestDir::new("modifier-extract-track");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![note_on(0, 0, 60, 100), note_off(12, 0, 60)],
                vec![note_on(0, 1, 72, 110), note_off(24, 1, 72)],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ExtractTrack(ExtractTrackTool { track_index: 1 }),
        )
        .expect("extract track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert_eq!(parsed.midi().track_count(), 1);
        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::NoteOn(note_on),
                },
                Delta {
                    delta: 24,
                    event: Event::NoteOff(note_off),
                },
            ] if note_on.channel == 1 && note_on.key == 72 && note_off.channel == 1 && note_off.key == 72
        ));
    }

    #[test]
    fn extract_track_rejects_out_of_range_track_indexes() {
        let dir = TestDir::new("modifier-extract-track-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ExtractTrack(ExtractTrackTool { track_index: 2 }),
        )
        .expect_err("out of range track selection should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
