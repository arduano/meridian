use std::path::Path;

use midi_toolkit::{
    events::Event,
    prelude::{EventSequenceExt, NoteSequenceExt},
    sequence::event::{Delta, merge_events},
};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct NoteLengthTool {
    pub min_ticks: Option<u64>,
    pub max_ticks: Option<u64>,
    pub scale: Option<f32>,
    pub fixed_ticks: Option<u64>,
}

pub(super) fn apply_note_length_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &NoteLengthTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Adjusting note lengths";
    progress.report(0, label)?;
    validate_note_length_tool(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed.midi().track_count();

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let iter = note_length_track_events(&parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn note_length_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a NoteLengthTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let note_events = adjusted_note_events(parsed, track_index, tool)?;
    let non_note_events = non_note_track_events(parsed, track_index)?;
    Some(merge_events(note_events, non_note_events))
}

fn adjusted_note_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a NoteLengthTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    Some(
        events
            .filter_note_events()
            .events_to_notes()
            .map(move |note| adjust_note_length(note, tool))
            .notes_to_events(),
    )
}

fn non_note_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(
        events
            .filter_non_note_events()
            .filter_map_events(map_non_track_start_event),
    )
}

fn adjust_note_length(
    note: Result<midi_toolkit::notes::Note<u64>, MeridianError>,
    tool: &NoteLengthTool,
) -> Result<midi_toolkit::notes::Note<u64>, MeridianError> {
    let mut note = note?;
    let mut len = note.len;

    if let Some(scale) = tool.scale {
        len = ((len as f64 * scale as f64).round() as i64).max(0) as u64;
    }
    if let Some(fixed_ticks) = tool.fixed_ticks {
        len = fixed_ticks;
    }
    if let Some(min_ticks) = tool.min_ticks {
        len = len.max(min_ticks);
    }
    if let Some(max_ticks) = tool.max_ticks {
        len = len.min(max_ticks);
    }

    note.len = len.max(1);
    Ok(note)
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn validate_note_length_tool(tool: &NoteLengthTool) -> Result<(), MeridianError> {
    if let Some(scale) = tool.scale
        && (!scale.is_finite() || scale < 0.0)
    {
        return Err(MeridianError::Validation(
            "note_length scale must be finite and non-negative".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::NoteLengthTool;
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn note_length_scales_and_clamps_note_lengths() {
        let dir = TestDir::new("modifier-note-length");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(10, 0, 60, 100), note_off(10, 0, 60)]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::NoteLength(NoteLengthTool {
                min_ticks: Some(15),
                max_ticks: Some(18),
                scale: Some(2.0),
                fixed_ticks: None,
            }),
        )
        .expect("note_length should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 10,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 18,
                    event: Event::NoteOff(_),
                },
            ]
        ));
    }

    #[test]
    fn note_length_rejects_invalid_scale() {
        let dir = TestDir::new("modifier-note-length-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::NoteLength(NoteLengthTool {
                min_ticks: None,
                max_ticks: None,
                scale: Some(f32::NAN),
                fixed_ticks: None,
            }),
        )
        .expect_err("invalid scale should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
