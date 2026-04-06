use std::path::Path;

use midi_toolkit::{
    events::{Event, MIDIEvent},
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
pub struct KeyMapTool {
    pub mappings: Vec<KeyMapEntry>,
    pub fold_to_range: Option<KeyRange>,
    pub drop_unmapped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct KeyMapEntry {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct KeyRange {
    pub min: u8,
    pub max: u8,
}

pub(super) fn apply_key_map_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &KeyMapTool,
) -> Result<(), MeridianError> {
    let parsed = ParsedMidiFile::load_from_file(input.to_path_buf())?;
    let writer = MIDIWriter::new(output.to_string_lossy().as_ref(), parsed.midi().ppq())
        .map_err(midi_write_error)?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = key_mapped_track_events(&parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    let mut writer = writer;
    writer.end().map_err(midi_write_error)?;
    Ok(())
}

fn key_mapped_track_events<'a>(
    parsed: &'a ParsedMidiFile,
    track_index: u32,
    tool: &'a KeyMapTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let track = parsed.midi().iter_track(track_index)?;
    let events = track.map(map_toolkit_event_result);

    // `filter_map_events` preserves delta timing across dropped events while letting the tool
    // rewrite the kept event in one pass.
    Some(events.filter_map_events(move |event| map_key_mapped_event(event, tool)))
}

fn map_key_mapped_event(mut event: Event, tool: &KeyMapTool) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Some(key) = event.key_mut() else {
        return Some(event);
    };

    if let Some(entry) = tool.mappings.iter().find(|entry| entry.from == *key) {
        *key = entry.to;
        return Some(event);
    }

    if let Some(range) = &tool.fold_to_range {
        *key = fold_key_to_range(*key, range);
        return Some(event);
    }

    if tool.drop_unmapped {
        None
    } else {
        Some(event)
    }
}

fn fold_key_to_range(mut key: u8, range: &KeyRange) -> u8 {
    let min = range.min.min(range.max);
    let max = range.max.max(range.min);
    let span = max.saturating_sub(min).saturating_add(1).max(1);

    while key < min {
        key = key.saturating_add(span);
    }

    while key > max {
        key = key.saturating_sub(span);
    }

    key
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::MIDIEvent;

    use super::{KeyMapEntry, KeyMapTool, KeyRange};
    use crate::midi::{
        MidiModifierTool,
        parsed::ParsedMidiFile,
        test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
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

        super::super::apply_modifier_tool_to_file(
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
}
