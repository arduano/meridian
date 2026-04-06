use std::path::Path;

use midi_toolkit::{
    events::{Event, TextEventKind},
    io::MIDIWriter,
    prelude::EventSequenceExt,
    sequence::event::Delta,
};

use crate::{
    error::MeridianError,
    midi::{
        modifier_tools::common::{
            map_toolkit_event_result, midi_write_error, write_try_track_events,
        },
        parsed::ParsedMidiFile,
        tools::{MetaTextTool, TextKind},
    },
};

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
