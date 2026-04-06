use std::path::Path;

use midi_toolkit::{
    events::Event,
    prelude::EventSequenceExt,
    sequence::event::{Delta, merge_events, merge_events_array},
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
pub struct SharedMetadataTrackTool {
    pub destination: SharedMetadataTrackDestination,
    pub move_tempo_events: bool,
    pub move_time_signatures: bool,
    pub move_key_signatures: bool,
    pub move_text_events: bool,
    pub move_unknown_meta_events: bool,
    pub move_channel_prefix_events: bool,
    pub move_midi_port_events: bool,
    pub move_control_change_events: bool,
    pub move_program_change_events: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SharedMetadataTrackDestination {
    #[default]
    CreateNew,
    InsertInto {
        track_index: usize,
    },
}

pub(super) fn apply_shared_metadata_track_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &SharedMetadataTrackTool,
) -> Result<(), MeridianError> {
    let parsed = load_parsed_midi(input)?;
    validate_shared_metadata_track_tool(&parsed, tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let has_selected_shared_events = has_selected_shared_events(&parsed, tool)?;

    if !tool.moves_any_events() || !has_selected_shared_events {
        write_all_body_tracks(&writer, &parsed, tool)?;
        return finish_midi_writer(writer);
    }

    match tool.destination {
        SharedMetadataTrackDestination::CreateNew => {
            write_try_track_events(&writer, merged_selected_shared_events(&parsed, tool))?;
            write_all_body_tracks(&writer, &parsed, tool)?;
        }
        SharedMetadataTrackDestination::InsertInto { track_index } => {
            for source_track_index in 0..parsed.midi().track_count() {
                if source_track_index == track_index {
                    let metadata = merged_selected_shared_events(&parsed, tool);
                    let body = body_track_events(&parsed, source_track_index as u32, tool)
                        .expect("track iteration should exist for a known track index");
                    write_try_track_events(&writer, merge_events(metadata, body))?;
                } else {
                    let body = body_track_events(&parsed, source_track_index as u32, tool)
                        .expect("track iteration should exist for a known track index");
                    write_try_track_events(&writer, body)?;
                }
            }
        }
    }

    finish_midi_writer(writer)
}

fn write_all_body_tracks(
    writer: &midi_toolkit::io::MIDIWriter,
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &SharedMetadataTrackTool,
) -> Result<(), MeridianError> {
    for track_index in 0..parsed.midi().track_count() {
        let iter = body_track_events(parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(writer, iter)?;
    }
    Ok(())
}

fn merged_selected_shared_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    tool: &'a SharedMetadataTrackTool,
) -> impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a {
    let iterators = (0..parsed.midi().track_count())
        .map(|track_index| {
            selected_shared_track_events(parsed, track_index as u32, tool)
                .expect("track iteration should exist for a known track index")
        })
        .collect::<Vec<_>>();
    merge_events_array(iterators)
}

fn selected_shared_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a SharedMetadataTrackTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| {
        selected_shared_event_class(&event, tool)
            .is_some()
            .then_some(event)
    }))
}

fn body_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a SharedMetadataTrackTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| {
        if matches!(event, Event::TrackStart(_))
            || selected_shared_event_class(&event, tool).is_some()
        {
            None
        } else {
            Some(event)
        }
    }))
}

fn has_selected_shared_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &SharedMetadataTrackTool,
) -> Result<bool, MeridianError> {
    for track_index in 0..parsed.midi().track_count() {
        let Some(events) = track_events(parsed, track_index as u32) else {
            continue;
        };
        for event in events {
            let event = event?;
            if selected_shared_event_class(&event.event, tool).is_some() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SharedTrackEventClass {
    Tempo,
    TimeSignature,
    KeySignature,
    Text,
    UnknownMeta,
    ChannelPrefix,
    MidiPort,
    ControlChange,
    ProgramChange,
}

impl SharedMetadataTrackTool {
    fn moves_any_events(&self) -> bool {
        self.move_tempo_events
            || self.move_time_signatures
            || self.move_key_signatures
            || self.move_text_events
            || self.move_unknown_meta_events
            || self.move_channel_prefix_events
            || self.move_midi_port_events
            || self.move_control_change_events
            || self.move_program_change_events
    }
}

fn validate_shared_metadata_track_tool(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &SharedMetadataTrackTool,
) -> Result<(), MeridianError> {
    match tool.destination {
        SharedMetadataTrackDestination::CreateNew => Ok(()),
        SharedMetadataTrackDestination::InsertInto { track_index } => {
            if track_index < parsed.midi().track_count() {
                Ok(())
            } else {
                Err(MeridianError::Validation(format!(
                    "shared_metadata_track insert track_index {} is out of range for {} tracks",
                    track_index,
                    parsed.midi().track_count()
                )))
            }
        }
    }
}

fn selected_shared_event_class(
    event: &Event,
    tool: &SharedMetadataTrackTool,
) -> Option<SharedTrackEventClass> {
    match event {
        Event::Tempo(_) if tool.move_tempo_events => Some(SharedTrackEventClass::Tempo),
        Event::TimeSignature(_) if tool.move_time_signatures => {
            Some(SharedTrackEventClass::TimeSignature)
        }
        Event::KeySignature(_) if tool.move_key_signatures => {
            Some(SharedTrackEventClass::KeySignature)
        }
        Event::Text(_) if tool.move_text_events => Some(SharedTrackEventClass::Text),
        Event::UnknownMeta(_) if tool.move_unknown_meta_events => {
            Some(SharedTrackEventClass::UnknownMeta)
        }
        Event::ChannelPrefix(_) if tool.move_channel_prefix_events => {
            Some(SharedTrackEventClass::ChannelPrefix)
        }
        Event::MIDIPort(_) if tool.move_midi_port_events => Some(SharedTrackEventClass::MidiPort),
        Event::ControlChange(_) if tool.move_control_change_events => {
            Some(SharedTrackEventClass::ControlChange)
        }
        Event::ProgramChange(_) if tool.move_program_change_events => {
            Some(SharedTrackEventClass::ProgramChange)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::{events::Event, sequence::event::Delta};

    use super::{SharedMetadataTrackDestination, SharedMetadataTrackTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{
                TestDir, key_signature, note_off, note_on, read_track_events, tempo, text,
                time_signature, write_toolkit_midi,
            },
        },
    };

    #[test]
    fn shared_metadata_track_moves_selected_metadata_into_target_track() {
        let dir = TestDir::new("modifier-shared-metadata");
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
                    time_signature(0, 4, 2, 24, 8),
                    key_signature(0, 0, 0),
                    note_on(7, 1, 64, 110),
                    note_off(10, 1, 64),
                ],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::InsertInto { track_index: 1 },
                move_tempo_events: true,
                move_time_signatures: true,
                move_key_signatures: true,
                move_text_events: true,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
            }),
        )
        .expect("shared_metadata_track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 2);

        let track0 = read_track_events(&parsed, 0);
        let track1 = read_track_events(&parsed, 1);

        assert!(matches!(
            track0.as_slice(),
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
        assert_eq!(track1.len(), 6);
        assert_eq!(
            track1
                .iter()
                .take(4)
                .filter(|event| {
                    matches!(
                        event.event,
                        Event::Tempo(_)
                            | Event::Text(_)
                            | Event::TimeSignature(_)
                            | Event::KeySignature(_)
                    )
                })
                .count(),
            4
        );
        assert_eq!(
            track1.iter().take(4).map(|event| event.delta).sum::<u64>(),
            0
        );
        assert!(matches!(
            track1[4],
            Delta {
                delta: 7,
                event: Event::NoteOn(_)
            }
        ));
        assert!(matches!(
            track1[5],
            Delta {
                delta: 10,
                event: Event::NoteOff(_)
            }
        ));
    }

    #[test]
    fn shared_metadata_track_can_create_a_new_first_track() {
        let dir = TestDir::new("modifier-shared-metadata-create-new");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(0, 500_000),
                note_on(5, 0, 60, 100),
                note_off(10, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::CreateNew,
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
            }),
        )
        .expect("shared_metadata_track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 2);
        let track0 = read_track_events(&parsed, 0);
        let track1 = read_track_events(&parsed, 1);

        assert!(matches!(
            track0.as_slice(),
            [Delta {
                delta: 0,
                event: Event::Tempo(_)
            },]
        ));
        assert!(matches!(
            track1.as_slice(),
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
    }

    #[test]
    fn shared_metadata_track_can_move_control_and_program_events() {
        let dir = TestDir::new("modifier-shared-metadata-controls");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_control_change_event(0, 0, 7, 100),
                Event::new_delta_program_change_event(0, 0, 42),
                note_on(5, 0, 60, 100),
                note_off(10, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::CreateNew,
                move_tempo_events: false,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: true,
                move_program_change_events: true,
            }),
        )
        .expect("shared_metadata_track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        assert_eq!(parsed.midi().track_count(), 2);
        let track0 = read_track_events(&parsed, 0);
        let track1 = read_track_events(&parsed, 1);

        assert!(matches!(
            track0.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::ControlChange(_)
                },
                Delta {
                    delta: 0,
                    event: Event::ProgramChange(_)
                },
            ]
        ));
        assert!(matches!(
            track1.as_slice(),
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
    }

    #[test]
    fn shared_metadata_track_rejects_invalid_insert_index() {
        let dir = TestDir::new("modifier-shared-metadata-invalid-index");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![tempo(0, 500_000)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::InsertInto { track_index: 1 },
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
            }),
        )
        .expect_err("invalid insert index should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
