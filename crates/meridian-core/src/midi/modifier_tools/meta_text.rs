use std::path::Path;

use midi_toolkit::{
    events::{Event, TextEventKind},
    io::MIDIWriter,
    prelude::EventSequenceExt,
    sequence::event::Delta,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::MeridianError,
    midi::{
        modifier_tools::common::{
            map_toolkit_event_result, midi_write_error, write_try_track_events,
        },
        parsed::ParsedMidiFile,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct MetaTextTool {
    pub keep_kinds: Vec<TextKind>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    Text,
    Copyright,
    TrackName,
    InstrumentName,
    Lyric,
    Marker,
    CuePoint,
    ProgramName,
    DeviceName,
    Undefined,
    MetaEvent,
}

pub(super) fn apply_meta_text_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &MetaTextTool,
) -> Result<(), MeridianError> {
    let parsed = ParsedMidiFile::load_from_file(input.to_path_buf())?;
    let writer = MIDIWriter::new(output.to_string_lossy().as_ref(), parsed.midi().ppq())
        .map_err(midi_write_error)?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = filtered_track_text_events(&parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    let mut writer = writer;
    writer.end().map_err(midi_write_error)?;
    Ok(())
}

fn filtered_track_text_events<'a>(
    parsed: &'a ParsedMidiFile,
    track_index: u32,
    tool: &'a MetaTextTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let track = parsed.midi().iter_track(track_index)?;
    let events = track.map(map_toolkit_event_result);

    Some(events.filter_map_events(move |event| map_text_event(event, tool)))
}

fn map_text_event(mut event: Event, tool: &MetaTextTool) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Event::Text(text) = &mut event else {
        return Some(event);
    };

    if !tool.keep_kinds.is_empty()
        && !tool
            .keep_kinds
            .iter()
            .any(|kind| text_kind_matches(text.kind, *kind))
    {
        return None;
    }

    Some(event)
}

fn text_kind_matches(kind: TextEventKind, expected: TextKind) -> bool {
    matches!(
        (kind, expected),
        (TextEventKind::TextEvent, TextKind::Text)
            | (TextEventKind::CopyrightNotice, TextKind::Copyright)
            | (TextEventKind::TrackName, TextKind::TrackName)
            | (TextEventKind::InstrumentName, TextKind::InstrumentName)
            | (TextEventKind::Lyric, TextKind::Lyric)
            | (TextEventKind::Marker, TextKind::Marker)
            | (TextEventKind::CuePoint, TextKind::CuePoint)
            | (TextEventKind::ProgramName, TextKind::ProgramName)
            | (TextEventKind::DeviceName, TextKind::DeviceName)
            | (TextEventKind::Undefined, TextKind::Undefined)
            | (TextEventKind::MetaEvent, TextKind::MetaEvent)
    )
}

#[cfg(test)]
mod tests {
    use midi_toolkit::{
        events::{Event, TextEvent, TextEventKind},
        sequence::event::Delta,
    };

    use super::{MetaTextTool, TextKind};
    use crate::midi::{
        MidiModifierTool,
        parsed::ParsedMidiFile,
        test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
    };

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

        super::super::apply_modifier_tool_to_file(
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
