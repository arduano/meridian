use std::path::Path;

use midi_toolkit::{
    events::{Event, MIDIEvent},
    prelude::EventSequenceExt,
    sequence::event::Delta,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::MeridianError,
    midi::{
        MIDI_KEY_COUNT,
        modifier_tools::common::{
            finish_midi_writer, load_parsed_midi, open_midi_writer, track_events,
            write_try_track_events,
        },
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
    let parsed = load_parsed_midi(input)?;
    let key_lookup = build_key_lookup(tool);
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = key_mapped_track_events(&parsed, track_index as u32, key_lookup)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    finish_midi_writer(writer)
}

fn key_mapped_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    key_lookup: [Option<u8>; MIDI_KEY_COUNT],
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;

    // `filter_map_events` preserves delta timing across dropped events while letting the tool
    // rewrite the kept event in one pass.
    Some(events.filter_map_events(move |event| map_key_mapped_event(event, key_lookup)))
}

fn map_key_mapped_event(
    mut event: Event,
    key_lookup: [Option<u8>; MIDI_KEY_COUNT],
) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Some(key) = event.key_mut() else {
        return Some(event);
    };

    *key = key_lookup[*key as usize]?;
    Some(event)
}

fn build_key_lookup(tool: &KeyMapTool) -> [Option<u8>; MIDI_KEY_COUNT] {
    let mut lookup = std::array::from_fn(|key| {
        let key = key as u8;
        if let Some(range) = &tool.fold_to_range {
            Some(fold_key_to_range(key, range))
        } else if tool.drop_unmapped {
            None
        } else {
            Some(key)
        }
    });

    for mapping in &tool.mappings {
        lookup[mapping.from as usize] = Some(mapping.to);
    }

    lookup
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
