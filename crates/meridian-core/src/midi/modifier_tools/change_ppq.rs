use std::path::Path;

use midi_toolkit::{events::Event, prelude::EventSequenceExt, sequence::event::Delta};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::MeridianError,
    midi::modifier_tools::common::{
        finish_midi_writer, load_parsed_midi, open_midi_writer, track_events,
        write_try_track_events,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ChangePpqTool {
    pub ppq: u16,
}

pub(super) fn apply_change_ppq_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &ChangePpqTool,
) -> Result<(), MeridianError> {
    validate_change_ppq_tool(tool)?;

    let parsed = load_parsed_midi(input)?;
    let writer = open_midi_writer(output, tool.ppq)?;

    for track_index in 0..parsed.midi().track_count() {
        let iter =
            scaled_ppq_track_events(&parsed, track_index as u32, parsed.midi().ppq(), tool.ppq)
                .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    finish_midi_writer(writer)
}

fn scaled_ppq_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    from_ppq: u16,
    to_ppq: u16,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(
        events
            .filter_map_events(map_non_track_start_event)
            .scale_event_ppq(from_ppq as u64, to_ppq as u64),
    )
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn validate_change_ppq_tool(tool: &ChangePpqTool) -> Result<(), MeridianError> {
    if tool.ppq == 0 {
        Err(MeridianError::Validation(
            "change_ppq ppq must be greater than zero".to_owned(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::ChangePpqTool;
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn change_ppq_scales_track_event_deltas() {
        let dir = TestDir::new("modifier-change-ppq");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(12, 0, 60, 100), note_off(24, 0, 60)]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChangePpq(ChangePpqTool { ppq: 192 }),
        )
        .expect("change ppq should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert_eq!(parsed.midi().ppq(), 192);
        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 24,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 48,
                    event: Event::NoteOff(_),
                },
            ]
        ));
    }

    #[test]
    fn change_ppq_rejects_zero_ppq() {
        let dir = TestDir::new("modifier-change-ppq-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChangePpq(ChangePpqTool { ppq: 0 }),
        )
        .expect_err("zero ppq should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
