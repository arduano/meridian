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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct SysexTool {
    pub strip_all: bool,
    pub prepend: Vec<Vec<u8>>,
}

pub(super) fn apply_sysex_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &SysexTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Rewriting SysEx events";
    progress.report(0, label)?;
    let prepended_sysex = build_prepended_sysex(tool);
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed
        .midi()
        .track_count()
        .max((!prepended_sysex.is_empty()) as usize);

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let source_track_exists = track_index < parsed.midi().track_count();

        if track_index == 0 && !prepended_sysex.is_empty() {
            let startup = prepended_sysex.iter().cloned().map(Ok);
            if source_track_exists {
                let body = sysex_track_events(&parsed, 0, tool.strip_all)
                    .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, startup.chain(body))?;
            } else {
                write_try_track_events(&writer, startup)?;
            }
            continue;
        }

        let body = sysex_track_events(&parsed, track_index as u32, tool.strip_all)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, body)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn sysex_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    strip_all: bool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| map_sysex_event(event, strip_all)))
}

fn map_sysex_event(event: Event, strip_all: bool) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_) if strip_all => None,
        event => Some(event),
    }
}

fn build_prepended_sysex(tool: &SysexTool) -> Vec<Delta<u64, Event>> {
    tool.prepend
        .iter()
        .map(|data| Event::new_delta_system_exclusive_message_event(0, data.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::SysexTool;
    use crate::midi::{
        MidiModifierTool,
        parsed::ParsedMidiFile,
        test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
    };

    #[test]
    fn sysex_pass_strips_existing_sysex_and_prepends_new_messages() {
        let dir = TestDir::new("modifier-sysex");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_system_exclusive_message_event(0, vec![1, 2, 3]),
                Event::new_delta_end_of_exclusive_event(4),
                note_on(6, 0, 60, 100),
                note_off(8, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Sysex(SysexTool {
                strip_all: true,
                prepend: vec![vec![9, 8, 7]],
            }),
        )
        .expect("sysex tool should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::SystemExclusiveMessage(message),
                },
                Delta {
                    delta: 10,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 8,
                    event: Event::NoteOff(_),
                },
            ] if message.data == vec![9, 8, 7]
        ));
    }
}
