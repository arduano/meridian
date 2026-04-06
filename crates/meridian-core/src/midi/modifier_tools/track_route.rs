use std::path::Path;

use midi_toolkit::{
    events::{Event, MIDIEvent},
    prelude::EventSequenceExt,
    sequence::event::{Delta, merge_events_array},
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

const SPLIT_CHANNEL_BUCKETS: usize = 17;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TrackRouteTool {
    CollapseAll,
    SplitByChannel,
    Map { mappings: Vec<TrackMapEntry> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TrackMapEntry {
    pub from: usize,
    pub to: usize,
}

pub(super) fn apply_track_route_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &TrackRouteTool,
) -> Result<(), MeridianError> {
    let parsed = load_parsed_midi(input)?;
    validate_track_route_tool(&parsed, tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    match tool {
        TrackRouteTool::CollapseAll => {
            let iter = merged_track_events_for_indices(&parsed, 0..parsed.midi().track_count())?;
            write_try_track_events(&writer, iter)?;
        }
        TrackRouteTool::SplitByChannel => {
            for bucket in 0..SPLIT_CHANNEL_BUCKETS {
                let mut iter = split_channel_track_events(&parsed, bucket).peekable();
                if iter.peek().is_some() {
                    write_try_track_events(&writer, iter)?;
                }
            }
        }
        TrackRouteTool::Map { mappings } => {
            let resolved_targets = build_track_route_targets(parsed.midi().track_count(), mappings);
            let max_target = resolved_targets.iter().copied().max().unwrap_or(0);
            for target_index in 0..=max_target {
                let source_indices =
                    resolved_targets
                        .iter()
                        .enumerate()
                        .filter_map(|(source_index, &target)| {
                            (target == target_index).then_some(source_index)
                        });
                let sources = source_indices.collect::<Vec<_>>();
                if sources.is_empty() {
                    continue;
                }
                let iter = merged_track_events_for_indices(&parsed, sources.into_iter())?;
                write_try_track_events(&writer, iter)?;
            }
        }
    }

    finish_midi_writer(writer)
}

fn merged_track_events_for_indices(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    source_indices: impl Iterator<Item = usize>,
) -> Result<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_, MeridianError> {
    let iterators = source_indices
        .map(|track_index| {
            preserved_track_events(parsed, track_index as u32).ok_or_else(|| {
                MeridianError::Validation(format!(
                    "track_route source track {} is out of range for {} tracks",
                    track_index,
                    parsed.midi().track_count()
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(merge_events_array(iterators))
}

fn split_channel_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    bucket: usize,
) -> impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_ {
    let iterators = (0..parsed.midi().track_count())
        .map(|track_index| {
            split_track_bucket_events(parsed, track_index as u32, bucket)
                .expect("track iteration should exist for a known track index")
        })
        .collect::<Vec<_>>();
    merge_events_array(iterators)
}

fn preserved_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(map_non_track_start_event))
}

fn split_track_bucket_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    bucket: usize,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| map_split_track_route_event(event, bucket)))
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

fn map_split_track_route_event(event: Event, bucket: usize) -> Option<Event> {
    let event = map_non_track_start_event(event)?;
    let route_bucket = event
        .channel()
        .map(|channel| channel as usize + 1)
        .unwrap_or(0);
    (route_bucket == bucket).then_some(event)
}

fn validate_track_route_tool(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &TrackRouteTool,
) -> Result<(), MeridianError> {
    let TrackRouteTool::Map { mappings } = tool else {
        return Ok(());
    };

    let mut seen = std::collections::HashSet::new();
    for mapping in mappings {
        if mapping.from >= parsed.midi().track_count() {
            return Err(MeridianError::Validation(format!(
                "track_route mapping source track {} is out of range for {} tracks",
                mapping.from,
                parsed.midi().track_count()
            )));
        }
        if !seen.insert(mapping.from) {
            return Err(MeridianError::Validation(format!(
                "track_route source track {} is mapped more than once",
                mapping.from
            )));
        }
    }

    Ok(())
}

fn build_track_route_targets(track_count: usize, mappings: &[TrackMapEntry]) -> Vec<usize> {
    let mut targets = (0..track_count).collect::<Vec<_>>();
    for mapping in mappings {
        targets[mapping.from] = mapping.to;
    }
    targets
}

#[cfg(test)]
mod tests {
    use midi_toolkit::{events::Event, sequence::event::Delta};

    use super::{TrackMapEntry, TrackRouteTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{
                TestDir, note_off, note_on, read_track_events, tempo, text, write_toolkit_midi,
            },
        },
    };

    #[test]
    fn track_route_collapse_all_merges_tracks() {
        let dir = TestDir::new("modifier-track-route-collapse");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![note_on(5, 0, 60, 100), note_off(10, 0, 60)],
                vec![note_on(7, 1, 64, 110), note_off(10, 1, 64)],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll),
        )
        .expect("track_route should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 1);
        let events = read_track_events(&parsed, 0);

        let mut absolute_tick = 0_u64;
        let note_ticks = events
            .into_iter()
            .map(|event| {
                absolute_tick = absolute_tick.saturating_add(event.delta);
                (absolute_tick, event.event)
            })
            .collect::<Vec<_>>();

        assert_eq!(note_ticks.len(), 4);
        assert!(matches!(note_ticks[0], (5, Event::NoteOn(_))));
        assert!(matches!(note_ticks[1], (7, Event::NoteOn(_))));
        assert!(matches!(note_ticks[2], (15, Event::NoteOff(_))));
        assert!(matches!(note_ticks[3], (17, Event::NoteOff(_))));
    }

    #[test]
    fn track_route_split_by_channel_keeps_non_channel_events_in_first_bucket() {
        let dir = TestDir::new("modifier-track-route-split");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![
                    tempo(0, 500_000),
                    note_on(5, 0, 60, 100),
                    note_off(10, 0, 60),
                ],
                vec![
                    text(0, "track"),
                    note_on(7, 1, 64, 110),
                    note_off(10, 1, 64),
                ],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TrackRoute(TrackRouteTool::SplitByChannel),
        )
        .expect("track_route should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 3);

        let bucket0 = read_track_events(&parsed, 0);
        let bucket1 = read_track_events(&parsed, 1);
        let bucket2 = read_track_events(&parsed, 2);

        assert_eq!(bucket0.len(), 2);
        assert_eq!(bucket0.iter().map(|event| event.delta).sum::<u64>(), 0);
        assert_eq!(
            bucket0
                .iter()
                .filter(|event| matches!(event.event, Event::Tempo(_) | Event::Text(_)))
                .count(),
            2
        );
        assert!(matches!(
            bucket1.as_slice(),
            [
                Delta {
                    delta: 5,
                    event: Event::NoteOn(_)
                },
                Delta {
                    delta: 10,
                    event: Event::NoteOff(_)
                },
            ]
        ));
        assert!(matches!(
            bucket2.as_slice(),
            [
                Delta {
                    delta: 7,
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
    fn track_route_map_routes_tracks_by_index() {
        let dir = TestDir::new("modifier-track-route-map");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![note_on(5, 0, 60, 100), note_off(10, 0, 60)],
                vec![note_on(7, 1, 64, 110), note_off(10, 1, 64)],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                mappings: vec![TrackMapEntry { from: 1, to: 0 }],
            }),
        )
        .expect("track_route should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 1);
        let events = read_track_events(&parsed, 0);

        let mut absolute_tick = 0_u64;
        let note_ticks = events
            .into_iter()
            .map(|event| {
                absolute_tick = absolute_tick.saturating_add(event.delta);
                (absolute_tick, event.event)
            })
            .collect::<Vec<_>>();

        assert_eq!(note_ticks.len(), 4);
        assert!(matches!(note_ticks[0], (5, Event::NoteOn(_))));
        assert!(matches!(note_ticks[1], (7, Event::NoteOn(_))));
        assert!(matches!(note_ticks[2], (15, Event::NoteOff(_))));
        assert!(matches!(note_ticks[3], (17, Event::NoteOff(_))));
    }

    #[test]
    fn track_route_rejects_duplicate_map_sources() {
        let dir = TestDir::new("modifier-track-route-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                mappings: vec![
                    TrackMapEntry { from: 0, to: 1 },
                    TrackMapEntry { from: 0, to: 2 },
                ],
            }),
        )
        .expect_err("duplicate mappings should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
