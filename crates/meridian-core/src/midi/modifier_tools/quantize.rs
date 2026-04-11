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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct QuantizeTool {
    pub rounding_ticks: u64,
    pub mode: QuantizeMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum QuantizeMode {
    #[default]
    NoteStartOnly,
    NoteStartAndEnd,
    AllEvents,
}

impl Default for QuantizeTool {
    fn default() -> Self {
        Self {
            rounding_ticks: 1,
            mode: QuantizeMode::NoteStartOnly,
        }
    }
}

pub(super) fn apply_quantize_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &QuantizeTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = match tool.mode {
        QuantizeMode::AllEvents => "Quantizing all events",
        QuantizeMode::NoteStartOnly | QuantizeMode::NoteStartAndEnd => "Quantizing notes",
    };
    progress.report(0, label)?;
    validate_quantize_tool(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed.midi().track_count();

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        match tool.mode {
            QuantizeMode::NoteStartOnly | QuantizeMode::NoteStartAndEnd => {
                let iter = quantized_note_track_events(
                    &parsed,
                    track_index as u32,
                    tool.rounding_ticks,
                    tool.mode,
                )
                .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, iter)?;
            }
            QuantizeMode::AllEvents => {
                let iter = quantized_all_event_track_events(
                    &parsed,
                    track_index as u32,
                    tool.rounding_ticks,
                )
                .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, iter)?;
            }
        }
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn quantized_note_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    rounding_ticks: u64,
    mode: QuantizeMode,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let note_events = quantized_note_events(parsed, track_index, rounding_ticks, mode)?;
    let non_note_events = non_note_track_events(parsed, track_index)?;
    Some(merge_events(note_events, non_note_events))
}

fn quantized_note_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    rounding_ticks: u64,
    mode: QuantizeMode,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(
        events
            .filter_note_events()
            .events_to_notes()
            .map(move |note| quantize_note(note, rounding_ticks, mode))
            .notes_to_events(),
    )
}

fn quantized_all_event_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    rounding_ticks: u64,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    let mut source_tick = 0_u64;
    let mut output_tick = 0_u64;

    Some(
        events
            .filter_map_events(map_non_track_start_event)
            .map(move |event| {
                let mut event = event?;
                source_tick = source_tick.saturating_add(event.delta);
                let rounded_tick = round_tick(source_tick, rounding_ticks);
                event.delta = rounded_tick.saturating_sub(output_tick);
                output_tick = rounded_tick;
                Ok(event)
            }),
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

fn quantize_note(
    note: Result<midi_toolkit::notes::Note<u64>, MeridianError>,
    rounding_ticks: u64,
    mode: QuantizeMode,
) -> Result<midi_toolkit::notes::Note<u64>, MeridianError> {
    let mut note = note?;
    let original_end = note.start.saturating_add(note.len);
    let quantized_start = round_tick(note.start, rounding_ticks);
    note.start = quantized_start;

    if matches!(mode, QuantizeMode::NoteStartAndEnd) {
        let quantized_end = round_tick(original_end, rounding_ticks);
        note.len = quantized_end.saturating_sub(quantized_start).max(1);
    }

    Ok(note)
}

fn round_tick(tick: u64, rounding_ticks: u64) -> u64 {
    let base = (tick / rounding_ticks) * rounding_ticks;
    let next = base.saturating_add(rounding_ticks);
    if tick - base <= next - tick {
        base
    } else {
        next
    }
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn validate_quantize_tool(tool: &QuantizeTool) -> Result<(), MeridianError> {
    if tool.rounding_ticks == 0 {
        Err(MeridianError::Validation(
            "quantize rounding_ticks must be greater than zero".to_owned(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::{QuantizeMode, QuantizeTool};
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
    fn quantize_rounds_note_starts_and_preserves_non_note_events() {
        let dir = TestDir::new("modifier-quantize");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(5, 500_000),
                note_on(8, 0, 60, 100),
                note_off(20, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Quantize(QuantizeTool {
                rounding_ticks: 10,
                mode: QuantizeMode::NoteStartOnly,
            }),
        )
        .expect("quantize should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 5,
                    event: Event::Tempo(tempo),
                },
                Delta {
                    delta: 5,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 20,
                    event: Event::NoteOff(_),
                },
            ] if tempo.tempo == 500_000
        ));
    }

    #[test]
    fn quantize_can_round_note_ends_too() {
        let dir = TestDir::new("modifier-quantize-note-end");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(8, 0, 60, 100), note_off(14, 0, 60)]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Quantize(QuantizeTool {
                rounding_ticks: 10,
                mode: QuantizeMode::NoteStartAndEnd,
            }),
        )
        .expect("quantize should succeed");

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
                    delta: 10,
                    event: Event::NoteOff(_),
                },
            ]
        ));
    }

    #[test]
    fn quantize_can_round_all_events() {
        let dir = TestDir::new("modifier-quantize-all-events");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(3, 500_000),
                note_on(5, 0, 60, 100),
                note_off(13, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Quantize(QuantizeTool {
                rounding_ticks: 10,
                mode: QuantizeMode::AllEvents,
            }),
        )
        .expect("quantize should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::Tempo(tempo),
                },
                Delta {
                    delta: 10,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 10,
                    event: Event::NoteOff(_),
                },
            ] if tempo.tempo == 500_000
        ));
    }

    #[test]
    fn quantize_rejects_zero_rounding_ticks() {
        let dir = TestDir::new("modifier-quantize-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Quantize(QuantizeTool {
                rounding_ticks: 0,
                mode: QuantizeMode::NoteStartOnly,
            }),
        )
        .expect_err("zero rounding_ticks should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
