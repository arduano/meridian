use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

use midi_toolkit::{
    events::Event,
    prelude::EventSequenceExt,
    sequence::event::{Delta, merge_events, merge_events_array},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    error::MeridianError,
    midi::{
        modifier_tools::common::{
            ToolProgress, finish_midi_writer, map_progress_range, open_midi_writer, track_events,
            write_try_track_events,
        },
        parsed::ParsedMidiFile,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct SharedMetadataTrackTool {
    pub destination: SharedMetadataTrackDestination,
    pub strip_redundant_events: bool,
    pub move_tempo_events: bool,
    pub move_time_signatures: bool,
    pub move_key_signatures: bool,
    pub move_text_events: bool,
    pub move_unknown_meta_events: bool,
    pub move_channel_prefix_events: bool,
    pub move_midi_port_events: bool,
    pub move_control_change_events: bool,
    pub move_program_change_events: bool,
    pub move_pitch_bend_events: bool,
    pub move_channel_pressure_events: bool,
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

pub(super) fn apply_shared_metadata_track_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &SharedMetadataTrackTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    progress.report(0, "Scanning shared events")?;
    validate_shared_metadata_track_tool(parsed, tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let has_selected_shared_events = has_selected_shared_events(parsed, tool)?;

    if !tool.moves_any_events() || !has_selected_shared_events {
        write_all_body_tracks(&writer, parsed, tool, progress, 20, 100)?;
        return finish_midi_writer(writer);
    }

    match tool.destination {
        SharedMetadataTrackDestination::CreateNew => {
            progress.report(30, "Building shared metadata track")?;
            write_try_track_events(&writer, merged_selected_shared_events(parsed, tool))?;
            write_all_body_tracks(&writer, parsed, tool, progress, 40, 100)?;
        }
        SharedMetadataTrackDestination::InsertInto { track_index } => {
            let track_count = parsed.midi().track_count();
            for source_track_index in 0..track_count {
                progress.report(
                    map_progress_range(40, 100, source_track_index, track_count),
                    "Writing body tracks",
                )?;
                if source_track_index == track_index {
                    let metadata = merged_selected_shared_events(parsed, tool);
                    let body = body_track_events(parsed, source_track_index as u32, tool)
                        .expect("track iteration should exist for a known track index");
                    write_try_track_events(&writer, merge_events(metadata, body))?;
                } else {
                    let body = body_track_events(parsed, source_track_index as u32, tool)
                        .expect("track iteration should exist for a known track index");
                    write_try_track_events(&writer, body)?;
                }
            }
            progress.report(100, "Writing body tracks")?;
        }
    }

    finish_midi_writer(writer)
}

fn write_all_body_tracks(
    writer: &midi_toolkit::io::MIDIWriter,
    parsed: &crate::midi::parsed::ParsedMidiFile,
    tool: &SharedMetadataTrackTool,
    progress: &mut ToolProgress<'_>,
    start_percent: u8,
    end_percent: u8,
) -> Result<(), MeridianError> {
    let track_count = parsed.midi().track_count();
    for track_index in 0..track_count {
        progress.report(
            map_progress_range(start_percent, end_percent, track_index, track_count),
            "Writing body tracks",
        )?;
        let iter = body_track_events(parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(writer, iter)?;
    }
    progress.report(end_percent, "Writing body tracks")?;
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
    normalize_redundant_shared_events(merge_events_array(iterators), tool.strip_redundant_events)
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
    PitchBend,
    ChannelPressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SharedEventRedundancyKey {
    Tempo,
    TimeSignature,
    KeySignature,
    ChannelPrefix,
    MidiPort,
    ProgramChange { channel: u8 },
    ControlChange { channel: u8, controller: u8 },
    PitchBend { channel: u8 },
    ChannelPressure { channel: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SharedEventStateValue {
    Tempo(u32),
    TimeSignature {
        numerator: u8,
        denominator: u8,
        ticks_per_click: u8,
        bb: u8,
    },
    KeySignature {
        sf: u8,
        mi: u8,
    },
    ChannelPrefix(u8),
    MidiPort(u8),
    ProgramChange(u8),
    ControlChange(u8),
    PitchBend(i16),
    ChannelPressure(u8),
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
            || self.move_pitch_bend_events
            || self.move_channel_pressure_events
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
        Event::PitchWheelChange(_) if tool.move_pitch_bend_events => {
            Some(SharedTrackEventClass::PitchBend)
        }
        Event::ChannelPressure(_) if tool.move_channel_pressure_events => {
            Some(SharedTrackEventClass::ChannelPressure)
        }
        _ => None,
    }
}

fn normalize_redundant_shared_events(
    events: impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>>,
    strip_redundant_events: bool,
) -> impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> {
    let mut events = events.fuse();
    let mut source_tick = 0_u64;
    let mut output_tick = 0_u64;
    let mut last_emitted_states = HashMap::new();
    let mut current_tick: Option<u64> = None;
    let mut current_events = Vec::new();
    let mut pending = VecDeque::new();
    let mut pending_error = None;

    std::iter::from_fn(move || {
        loop {
            if let Some(event) = pending.pop_front() {
                return Some(event);
            }

            if let Some(error) = pending_error.take() {
                return Some(Err(error));
            }

            match events.next() {
                Some(Ok(event)) => {
                    source_tick = source_tick.saturating_add(event.delta);
                    match current_tick {
                        None => {
                            current_tick = Some(source_tick);
                            current_events.push(event.event);
                        }
                        Some(tick) if tick == source_tick => current_events.push(event.event),
                        Some(tick) => {
                            queue_normalized_tick_events(
                                &mut pending,
                                &mut output_tick,
                                &mut last_emitted_states,
                                tick,
                                std::mem::take(&mut current_events),
                                strip_redundant_events,
                            );
                            current_tick = Some(source_tick);
                            current_events.push(event.event);
                        }
                    }
                }
                Some(Err(error)) => {
                    if let Some(tick) = current_tick.take() {
                        queue_normalized_tick_events(
                            &mut pending,
                            &mut output_tick,
                            &mut last_emitted_states,
                            tick,
                            std::mem::take(&mut current_events),
                            strip_redundant_events,
                        );
                        pending_error = Some(error);
                    } else {
                        return Some(Err(error));
                    }
                }
                None => {
                    if let Some(tick) = current_tick.take() {
                        queue_normalized_tick_events(
                            &mut pending,
                            &mut output_tick,
                            &mut last_emitted_states,
                            tick,
                            std::mem::take(&mut current_events),
                            strip_redundant_events,
                        );
                    } else {
                        return None;
                    }
                }
            }
        }
    })
}

fn queue_normalized_tick_events(
    pending: &mut VecDeque<Result<Delta<u64, Event>, MeridianError>>,
    output_tick: &mut u64,
    last_emitted_states: &mut HashMap<SharedEventRedundancyKey, SharedEventStateValue>,
    tick: u64,
    events: Vec<Event>,
    strip_redundant_events: bool,
) {
    let normalized = if strip_redundant_events {
        normalize_tick_events(events, last_emitted_states)
    } else {
        events
    };
    let emitted_any = !normalized.is_empty();
    for (index, event) in normalized.into_iter().enumerate() {
        let delta = if index == 0 {
            tick.saturating_sub(*output_tick)
        } else {
            0
        };
        pending.push_back(Ok(Delta::new(delta, event)));
    }
    if emitted_any {
        *output_tick = tick;
    }
}

fn normalize_tick_events(
    events: Vec<Event>,
    last_emitted_states: &mut HashMap<SharedEventRedundancyKey, SharedEventStateValue>,
) -> Vec<Event> {
    let mut last_indices = HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if let Some(key) = redundant_shared_event_key(event) {
            last_indices.insert(key, index);
        }
    }

    events
        .into_iter()
        .enumerate()
        .filter_map(|(index, event)| {
            let Some(key) = redundant_shared_event_key(&event) else {
                return Some(event);
            };
            if last_indices.get(&key).copied() != Some(index) {
                return None;
            }

            let Some(state) = redundant_shared_event_state(&event) else {
                return Some(event);
            };
            if last_emitted_states.get(&key).copied() == Some(state) {
                return None;
            }

            last_emitted_states.insert(key, state);
            Some(event)
        })
        .collect()
}

fn redundant_shared_event_key(event: &Event) -> Option<SharedEventRedundancyKey> {
    match event {
        Event::Tempo(_) => Some(SharedEventRedundancyKey::Tempo),
        Event::TimeSignature(_) => Some(SharedEventRedundancyKey::TimeSignature),
        Event::KeySignature(_) => Some(SharedEventRedundancyKey::KeySignature),
        Event::ChannelPrefix(_) => Some(SharedEventRedundancyKey::ChannelPrefix),
        Event::MIDIPort(_) => Some(SharedEventRedundancyKey::MidiPort),
        Event::ProgramChange(program) => Some(SharedEventRedundancyKey::ProgramChange {
            channel: program.channel,
        }),
        Event::ControlChange(control) => Some(SharedEventRedundancyKey::ControlChange {
            channel: control.channel,
            controller: control.controller,
        }),
        Event::PitchWheelChange(bend) => Some(SharedEventRedundancyKey::PitchBend {
            channel: bend.channel,
        }),
        Event::ChannelPressure(pressure) => Some(SharedEventRedundancyKey::ChannelPressure {
            channel: pressure.channel,
        }),
        _ => None,
    }
}

fn redundant_shared_event_state(event: &Event) -> Option<SharedEventStateValue> {
    match event {
        Event::Tempo(tempo) => Some(SharedEventStateValue::Tempo(tempo.tempo)),
        Event::TimeSignature(signature) => Some(SharedEventStateValue::TimeSignature {
            numerator: signature.numerator,
            denominator: signature.denominator,
            ticks_per_click: signature.ticks_per_click,
            bb: signature.bb,
        }),
        Event::KeySignature(signature) => Some(SharedEventStateValue::KeySignature {
            sf: signature.sf,
            mi: signature.mi,
        }),
        Event::ChannelPrefix(prefix) => Some(SharedEventStateValue::ChannelPrefix(prefix.channel)),
        Event::MIDIPort(port) => Some(SharedEventStateValue::MidiPort(port.channel)),
        Event::ProgramChange(program) => {
            Some(SharedEventStateValue::ProgramChange(program.program))
        }
        Event::ControlChange(control) => Some(SharedEventStateValue::ControlChange(control.value)),
        Event::PitchWheelChange(bend) => Some(SharedEventStateValue::PitchBend(bend.pitch)),
        Event::ChannelPressure(pressure) => {
            Some(SharedEventStateValue::ChannelPressure(pressure.pressure))
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
                strip_redundant_events: false,
                move_tempo_events: true,
                move_time_signatures: true,
                move_key_signatures: true,
                move_text_events: true,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
                move_pitch_bend_events: false,
                move_channel_pressure_events: false,
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
                strip_redundant_events: false,
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
                move_pitch_bend_events: false,
                move_channel_pressure_events: false,
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
                strip_redundant_events: false,
                move_tempo_events: false,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: true,
                move_program_change_events: true,
                move_pitch_bend_events: false,
                move_channel_pressure_events: false,
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
                strip_redundant_events: false,
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: false,
                move_program_change_events: false,
                move_pitch_bend_events: false,
                move_channel_pressure_events: false,
            }),
        )
        .expect_err("invalid insert index should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }

    #[test]
    fn shared_metadata_track_can_strip_redundant_state_events_on_same_tick() {
        let dir = TestDir::new("modifier-shared-metadata-normalize");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(0, 500_000),
                tempo(0, 600_000),
                Event::new_delta_control_change_event(0, 0, 7, 10),
                Event::new_delta_control_change_event(0, 0, 7, 20),
                Event::new_delta_program_change_event(0, 0, 1),
                Event::new_delta_program_change_event(0, 0, 2),
                note_on(5, 0, 60, 100),
                note_off(10, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::CreateNew,
                strip_redundant_events: true,
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: true,
                move_program_change_events: true,
                move_pitch_bend_events: false,
                move_channel_pressure_events: false,
            }),
        )
        .expect("shared_metadata_track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let track0 = read_track_events(&parsed, 0);

        assert_eq!(track0.len(), 3);
        assert!(matches!(
            track0[0],
            Delta {
                delta: 0,
                event: Event::Tempo(ref tempo)
            } if tempo.tempo == 600_000
        ));
        assert!(matches!(
            track0[1],
            Delta {
                delta: 0,
                event: Event::ControlChange(ref control)
            } if control.channel == 0 && control.controller == 7 && control.value == 20
        ));
        assert!(matches!(
            track0[2],
            Delta {
                delta: 0,
                event: Event::ProgramChange(ref program)
            } if program.channel == 0 && program.program == 2
        ));
    }

    #[test]
    fn shared_metadata_track_can_strip_redundant_state_events_across_ticks() {
        let dir = TestDir::new("modifier-shared-metadata-normalize-across-ticks");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(0, 500_000),
                Event::new_delta_control_change_event(0, 0, 7, 64),
                Event::new_delta_pitch_wheel_change_event(0, 0, 1000),
                Event::new_delta_channel_pressure_event(0, 0, 50),
                tempo(5, 500_000),
                Event::new_delta_control_change_event(0, 0, 7, 64),
                Event::new_delta_pitch_wheel_change_event(0, 0, 1000),
                Event::new_delta_channel_pressure_event(0, 0, 50),
                tempo(5, 600_000),
                Event::new_delta_control_change_event(0, 0, 7, 96),
                Event::new_delta_pitch_wheel_change_event(0, 0, 2000),
                Event::new_delta_channel_pressure_event(0, 0, 80),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::SharedMetadataTrack(SharedMetadataTrackTool {
                destination: SharedMetadataTrackDestination::CreateNew,
                strip_redundant_events: true,
                move_tempo_events: true,
                move_time_signatures: false,
                move_key_signatures: false,
                move_text_events: false,
                move_unknown_meta_events: false,
                move_channel_prefix_events: false,
                move_midi_port_events: false,
                move_control_change_events: true,
                move_program_change_events: false,
                move_pitch_bend_events: true,
                move_channel_pressure_events: true,
            }),
        )
        .expect("shared_metadata_track should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let track0 = read_track_events(&parsed, 0);

        assert_eq!(track0.len(), 8);
        assert!(matches!(
            track0[0],
            Delta {
                delta: 0,
                event: Event::Tempo(ref tempo)
            } if tempo.tempo == 500_000
        ));
        assert!(matches!(
            track0[1],
            Delta {
                delta: 0,
                event: Event::ControlChange(ref control)
            } if control.channel == 0 && control.controller == 7 && control.value == 64
        ));
        assert!(matches!(
            track0[2],
            Delta {
                delta: 0,
                event: Event::PitchWheelChange(ref bend)
            } if bend.channel == 0 && bend.pitch == 1000
        ));
        assert!(matches!(
            track0[3],
            Delta {
                delta: 0,
                event: Event::ChannelPressure(ref pressure)
            } if pressure.channel == 0 && pressure.pressure == 50
        ));
        assert!(matches!(
            track0[4],
            Delta {
                delta: 10,
                event: Event::Tempo(ref tempo)
            } if tempo.tempo == 600_000
        ));
        assert!(matches!(
            track0[5],
            Delta {
                delta: 0,
                event: Event::ControlChange(ref control)
            } if control.channel == 0 && control.controller == 7 && control.value == 96
        ));
        assert!(matches!(
            track0[6],
            Delta {
                delta: 0,
                event: Event::PitchWheelChange(ref bend)
            } if bend.channel == 0 && bend.pitch == 2000
        ));
        assert!(matches!(
            track0[7],
            Delta {
                delta: 0,
                event: Event::ChannelPressure(ref pressure)
            } if pressure.channel == 0 && pressure.pressure == 80
        ));
    }
}
