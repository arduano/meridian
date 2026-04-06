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
    midi::modifier_tools::common::{
        finish_midi_writer, load_parsed_midi, open_midi_writer, track_events,
        write_try_track_events,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct RangeSelectTool {
    pub start_ticks: u64,
    pub end_ticks: u64,
    pub offset_ticks: Option<u64>,
    pub track_select: Option<usize>,
    pub preserve_system_events: bool,
    pub edge_behavior: RangeEdgeBehavior,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum RangeEdgeBehavior {
    Keep,
    Skip,
    #[default]
    Trim,
}

pub(super) fn apply_range_select_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &RangeSelectTool,
) -> Result<(), MeridianError> {
    let parsed = load_parsed_midi(input)?;
    validate_range_select_tool(&parsed, tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    match tool.track_select {
        Some(track_index) => {
            let iter = selected_track_events(&parsed, track_index as u32, tool)
                .expect("track iteration should exist for a validated track index");
            write_try_track_events(&writer, iter)?;
        }
        None => {
            for track_index in 0..parsed.midi().track_count() {
                let iter = selected_track_events(&parsed, track_index as u32, tool)
                    .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, iter)?;
            }
        }
    }

    finish_midi_writer(writer)
}

fn selected_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a RangeSelectTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let note_events = selected_note_events(parsed, track_index, tool)?;
    let non_note_events = selected_non_note_events(parsed, track_index, tool)?;
    Some(merge_events(note_events, non_note_events))
}

fn selected_note_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a RangeSelectTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    let output_start = output_start_tick(tool);

    Some(
        events
            .filter_note_events()
            .events_to_notes()
            .filter_map(move |note| select_note(note, tool, output_start))
            .notes_to_events(),
    )
}

fn selected_non_note_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a RangeSelectTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    let mut absolute_tick = 0_u64;
    let mut output_tick = 0_u64;
    let output_start = output_start_tick(tool);

    Some(
        events
            .filter_non_note_events()
            .filter_map_events(map_non_track_start_event)
            .filter_map(move |event| {
                let mut event = match event {
                    Ok(event) => event,
                    Err(error) => return Some(Err(error)),
                };

                absolute_tick = absolute_tick.saturating_add(event.delta);
                let output_absolute_tick =
                    if should_keep_non_note_event(&event.event, absolute_tick, tool) {
                        map_output_tick_for_non_note(
                            &event.event,
                            absolute_tick,
                            output_start,
                            tool,
                        )
                    } else {
                        return None;
                    };

                event.delta = output_absolute_tick.saturating_sub(output_tick);
                output_tick = output_absolute_tick;
                Some(Ok(event))
            }),
    )
}

fn select_note(
    note: Result<midi_toolkit::notes::Note<u64>, MeridianError>,
    tool: &RangeSelectTool,
    output_start: u64,
) -> Option<Result<midi_toolkit::notes::Note<u64>, MeridianError>> {
    let mut note = match note {
        Ok(note) => note,
        Err(error) => return Some(Err(error)),
    };

    let note_start = note.start;
    let note_end = note.start.saturating_add(note.len);
    if note_end <= tool.start_ticks || note_start >= tool.end_ticks {
        return None;
    }

    match tool.edge_behavior {
        RangeEdgeBehavior::Skip => {
            if note_start < tool.start_ticks || note_end > tool.end_ticks {
                return None;
            }
        }
        RangeEdgeBehavior::Trim => {
            let trimmed_start = note_start.max(tool.start_ticks);
            let trimmed_end = note_end.min(tool.end_ticks);
            if trimmed_end <= trimmed_start {
                return None;
            }
            note.start = trimmed_start;
            note.len = trimmed_end.saturating_sub(trimmed_start);
        }
        RangeEdgeBehavior::Keep => {}
    }

    note.start = note
        .start
        .saturating_sub(tool.start_ticks)
        .saturating_add(output_start);

    Some(Ok(note))
}

fn should_keep_non_note_event(event: &Event, absolute_tick: u64, tool: &RangeSelectTool) -> bool {
    if tick_in_selected_range(absolute_tick, tool) {
        return true;
    }

    tool.preserve_system_events
        && absolute_tick < tool.start_ticks
        && is_preserved_system_event(event)
}

fn map_output_tick_for_non_note(
    event: &Event,
    absolute_tick: u64,
    output_start: u64,
    tool: &RangeSelectTool,
) -> u64 {
    if tool.preserve_system_events
        && absolute_tick < tool.start_ticks
        && is_preserved_system_event(event)
    {
        output_start
    } else {
        absolute_tick
            .saturating_sub(tool.start_ticks)
            .saturating_add(output_start)
    }
}

fn is_preserved_system_event(event: &Event) -> bool {
    matches!(
        event,
        Event::SystemExclusiveMessage(_)
            | Event::Undefined(_)
            | Event::SongPositionPointer(_)
            | Event::SongSelect(_)
            | Event::TuneRequest(_)
            | Event::EndOfExclusive(_)
            | Event::Text(_)
            | Event::UnknownMeta(_)
            | Event::Color(_)
            | Event::ChannelPrefix(_)
            | Event::MIDIPort(_)
            | Event::Tempo(_)
            | Event::SMPTEOffset(_)
            | Event::TimeSignature(_)
            | Event::KeySignature(_)
    )
}

fn tick_in_selected_range(tick: u64, tool: &RangeSelectTool) -> bool {
    tick >= tool.start_ticks && tick < tool.end_ticks
}

fn output_start_tick(tool: &RangeSelectTool) -> u64 {
    tool.offset_ticks.unwrap_or(tool.start_ticks)
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn validate_range_select_tool(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &RangeSelectTool,
) -> Result<(), MeridianError> {
    if tool.start_ticks >= tool.end_ticks {
        return Err(MeridianError::Validation(
            "range_select start_ticks must be less than end_ticks".to_owned(),
        ));
    }

    if let Some(track_index) = tool.track_select
        && track_index >= parsed.midi().track_count()
    {
        return Err(MeridianError::Validation(format!(
            "range_select track_select {} is out of range for {} tracks",
            track_index,
            parsed.midi().track_count()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use midi_toolkit::{events::Event, sequence::event::Delta};

    use super::{RangeEdgeBehavior, RangeSelectTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{
                TestDir, note_off, note_on, read_track_events, tempo, write_toolkit_midi,
            },
        },
    };

    #[test]
    fn range_select_trims_notes_and_shifts_to_offset() {
        let dir = TestDir::new("modifier-range-select-trim");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(10, 0, 60, 100), note_off(20, 0, 60)]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool {
                start_ticks: 15,
                end_ticks: 25,
                offset_ticks: Some(0),
                track_select: None,
                preserve_system_events: false,
                edge_behavior: RangeEdgeBehavior::Trim,
            }),
        )
        .expect("range_select should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::NoteOn(_)
                },
                Delta {
                    delta: 10,
                    event: Event::NoteOff(_)
                },
            ]
        ));
    }

    #[test]
    fn range_select_skip_drops_edge_crossing_notes() {
        let dir = TestDir::new("modifier-range-select-skip");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(10, 0, 60, 100), note_off(20, 0, 60)]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool {
                start_ticks: 15,
                end_ticks: 25,
                offset_ticks: Some(0),
                track_select: None,
                preserve_system_events: false,
                edge_behavior: RangeEdgeBehavior::Skip,
            }),
        )
        .expect("range_select should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert!(read_track_events(&parsed, 0).is_empty());
    }

    #[test]
    fn range_select_can_preserve_pre_range_system_events() {
        let dir = TestDir::new("modifier-range-select-system");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(5, 500_000),
                note_on(10, 0, 60, 100),
                note_off(20, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool {
                start_ticks: 20,
                end_ticks: 40,
                offset_ticks: Some(0),
                track_select: None,
                preserve_system_events: true,
                edge_behavior: RangeEdgeBehavior::Trim,
            }),
        )
        .expect("range_select should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert_eq!(events.len(), 3);
        assert_eq!(
            events.iter().take(2).map(|event| event.delta).sum::<u64>(),
            0
        );
        assert_eq!(
            events
                .iter()
                .take(2)
                .filter(|event| matches!(event.event, Event::Tempo(_) | Event::NoteOn(_)))
                .count(),
            2
        );
        let mut absolute_tick = 0_u64;
        let note_off_ticks = events
            .iter()
            .filter_map(|event| {
                absolute_tick = absolute_tick.saturating_add(event.delta);
                matches!(event.event, Event::NoteOff(_)).then_some(absolute_tick)
            })
            .collect::<Vec<_>>();
        assert_eq!(note_off_ticks, vec![15]);
    }

    #[test]
    fn range_select_can_output_only_one_track() {
        let dir = TestDir::new("modifier-range-select-track");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![note_on(10, 0, 60, 100), note_off(10, 0, 60)],
                vec![note_on(10, 1, 64, 110), note_off(10, 1, 64)],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool {
                start_ticks: 0,
                end_ticks: 40,
                offset_ticks: Some(0),
                track_select: Some(1),
                preserve_system_events: false,
                edge_behavior: RangeEdgeBehavior::Trim,
            }),
        )
        .expect("range_select should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 1);
        let events = read_track_events(&parsed, 0);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Delta { delta: 10, event: Event::NoteOn(ref note) } if note.channel == 1
        ));
        assert!(matches!(
            events[1],
            Delta { delta: 10, event: Event::NoteOff(ref note) } if note.channel == 1
        ));
    }

    #[test]
    fn range_select_rejects_invalid_bounds() {
        let dir = TestDir::new("modifier-range-select-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::RangeSelect(RangeSelectTool {
                start_ticks: 10,
                end_ticks: 10,
                offset_ticks: None,
                track_select: None,
                preserve_system_events: false,
                edge_behavior: RangeEdgeBehavior::Trim,
            }),
        )
        .expect_err("invalid range should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
