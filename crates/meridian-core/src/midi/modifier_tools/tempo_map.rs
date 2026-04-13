use std::path::Path;

use midi_toolkit::{
    events::Event,
    prelude::EventSequenceExt,
    sequence::event::{Delta, merge_events},
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TempoMapTool {
    Flatten {
        tempo: u32,
    },
    ScaleBpm {
        factor: f64,
    },
    Replace {
        points: Vec<TempoPoint>,
        #[serde(default)]
        destination: TempoMapDestination,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TempoPoint {
    pub tick: u64,
    pub tempo: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum TempoMapDestination {
    #[default]
    InjectIntoFirstTrack,
    CreateNewTempoTrack,
}

pub(super) fn apply_tempo_map_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &TempoMapTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    validate_tempo_map_tool(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    match tool {
        TempoMapTool::Flatten { tempo } => {
            let label = "Rewriting tempo map";
            progress.report(0, label)?;
            let flattened_tempo = [Event::new_delta_tempo_event(0, *tempo)];
            let track_count = parsed.midi().track_count().max(1);

            for track_index in 0..track_count {
                progress.report_steps_completed(track_index, track_count, label)?;
                let source_track_exists = track_index < parsed.midi().track_count();
                if track_index == 0 {
                    let startup = flattened_tempo.iter().cloned().map(Ok);
                    if source_track_exists {
                        let body = non_tempo_track_events(parsed, 0)
                            .expect("track iteration should exist for a known track index");
                        write_try_track_events(&writer, startup.chain(body))?;
                    } else {
                        write_try_track_events(&writer, startup)?;
                    }
                } else {
                    let body = non_tempo_track_events(parsed, track_index as u32)
                        .expect("track iteration should exist for a known track index");
                    write_try_track_events(&writer, body)?;
                }
            }
            progress.report(100, label)?;
        }
        TempoMapTool::ScaleBpm { factor } => {
            let label = "Rewriting tempo map";
            let track_count = parsed.midi().track_count();
            progress.report(0, label)?;
            for track_index in 0..track_count {
                progress.report_steps_completed(track_index, track_count, label)?;
                let iter = scaled_tempo_track_events(parsed, track_index as u32, *factor)
                    .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, iter)?;
            }
            progress.report(100, label)?;
        }
        TempoMapTool::Replace {
            points,
            destination,
        } => {
            progress.report(0, "Building replacement tempo events")?;
            let replacement_tempos = build_replacement_tempo_events(points);

            match destination {
                TempoMapDestination::InjectIntoFirstTrack => {
                    let label = "Rewriting tempo map";
                    let track_count = parsed.midi().track_count().max(1);

                    for track_index in 0..track_count {
                        progress
                            .report(map_progress_range(10, 100, track_index, track_count), label)?;
                        let source_track_exists = track_index < parsed.midi().track_count();
                        if track_index == 0 {
                            let replacement = replacement_tempos.iter().cloned().map(Ok);
                            if source_track_exists {
                                let body = non_tempo_track_events(parsed, 0)
                                    .expect("track iteration should exist for a known track index");
                                write_try_track_events(&writer, merge_events(body, replacement))?;
                            } else {
                                write_try_track_events(&writer, replacement)?;
                            }
                        } else {
                            let body = non_tempo_track_events(parsed, track_index as u32)
                                .expect("track iteration should exist for a known track index");
                            write_try_track_events(&writer, body)?;
                        }
                    }
                    progress.report(100, label)?;
                }
                TempoMapDestination::CreateNewTempoTrack => {
                    progress.report(10, "Writing tempo track")?;
                    write_try_track_events(&writer, replacement_tempos.iter().cloned().map(Ok))?;
                    let track_count = parsed.midi().track_count();
                    let label = "Rewriting tempo map";
                    for track_index in 0..track_count {
                        progress
                            .report(map_progress_range(20, 100, track_index, track_count), label)?;
                        let body = non_tempo_track_events(parsed, track_index as u32)
                            .expect("track iteration should exist for a known track index");
                        write_try_track_events(&writer, body)?;
                    }
                    progress.report(100, label)?;
                }
            }
        }
    }

    finish_midi_writer(writer)
}

fn non_tempo_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(map_non_tempo_event))
}

fn scaled_tempo_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    factor: f64,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| map_scaled_tempo_event(event, factor)))
}

fn map_non_tempo_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) | Event::Tempo(_) => None,
        event => Some(event),
    }
}

fn map_scaled_tempo_event(mut event: Event, factor: f64) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    if let Event::Tempo(tempo) = &mut event {
        let factor = factor.max(0.0001);
        tempo.tempo = ((tempo.tempo as f64 / factor).round() as i64).clamp(1, 0xFF_FFFF) as u32;
    }

    Some(event)
}

fn build_replacement_tempo_events(points: &[TempoPoint]) -> Vec<Delta<u64, Event>> {
    let mut sorted_points = points.to_vec();
    sorted_points.sort_by_key(|point| point.tick);

    let mut previous_tick = 0;
    sorted_points
        .into_iter()
        .map(|point| {
            let delta = point.tick.saturating_sub(previous_tick);
            previous_tick = point.tick;
            Event::new_delta_tempo_event(delta, point.tempo)
        })
        .collect()
}

fn validate_tempo_map_tool(tool: &TempoMapTool) -> Result<(), MeridianError> {
    match tool {
        TempoMapTool::Flatten { tempo } => validate_tempo_value(*tempo),
        TempoMapTool::ScaleBpm { factor } => {
            if factor.is_finite() {
                Ok(())
            } else {
                Err(MeridianError::Validation(
                    "tempo_map scale_bpm factor must be finite".to_owned(),
                ))
            }
        }
        TempoMapTool::Replace { points, .. } => {
            for point in points {
                validate_tempo_value(point.tempo)?;
            }
            Ok(())
        }
    }
}

fn validate_tempo_value(tempo: u32) -> Result<(), MeridianError> {
    if (1..=0xFF_FFFF).contains(&tempo) {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "tempo_map tempo {} is out of range 1..{}",
            tempo, 0xFF_FFFF
        )))
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::{TempoMapDestination, TempoMapTool, TempoPoint};
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
    fn tempo_map_replace_injects_replacement_tempo_events_into_first_track() {
        let dir = TestDir::new("modifier-tempo-map-replace");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(0, 500_000),
                note_on(12, 0, 60, 100),
                note_off(24, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TempoMap(TempoMapTool::Replace {
                points: vec![
                    TempoPoint {
                        tick: 24,
                        tempo: 400_000,
                    },
                    TempoPoint {
                        tick: 0,
                        tempo: 600_000,
                    },
                ],
                destination: TempoMapDestination::InjectIntoFirstTrack,
            }),
        )
        .expect("tempo map should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::Tempo(first),
                },
                Delta {
                    delta: 12,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 12,
                    event: Event::Tempo(second),
                },
                Delta {
                    delta: 12,
                    event: Event::NoteOff(_),
                },
            ] if first.tempo == 600_000 && second.tempo == 400_000
        ));
    }

    #[test]
    fn tempo_map_replace_can_write_to_a_new_tempo_track() {
        let dir = TestDir::new("modifier-tempo-map-replace-new-track");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[
                vec![note_on(12, 0, 60, 100), note_off(24, 0, 60)],
                vec![note_on(6, 1, 72, 110), note_off(18, 1, 72)],
            ],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TempoMap(TempoMapTool::Replace {
                points: vec![TempoPoint {
                    tick: 0,
                    tempo: 600_000,
                }],
                destination: TempoMapDestination::CreateNewTempoTrack,
            }),
        )
        .expect("tempo map should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let tempo_track = read_track_events(&parsed, 0);
        let first_body_track = read_track_events(&parsed, 1);
        let second_body_track = read_track_events(&parsed, 2);

        assert_eq!(parsed.midi().track_count(), 3);
        assert!(matches!(
            tempo_track.as_slice(),
            [Delta {
                delta: 0,
                event: Event::Tempo(tempo),
            }] if tempo.tempo == 600_000
        ));
        assert!(matches!(
            first_body_track.as_slice(),
            [
                Delta {
                    delta: 12,
                    event: Event::NoteOn(note_on),
                },
                Delta {
                    delta: 24,
                    event: Event::NoteOff(note_off),
                },
            ] if note_on.channel == 0 && note_off.channel == 0
        ));
        assert!(matches!(
            second_body_track.as_slice(),
            [
                Delta {
                    delta: 6,
                    event: Event::NoteOn(note_on),
                },
                Delta {
                    delta: 18,
                    event: Event::NoteOff(note_off),
                },
            ] if note_on.channel == 1 && note_off.channel == 1
        ));
    }

    #[test]
    fn tempo_map_rejects_invalid_tempo_values() {
        let dir = TestDir::new("modifier-tempo-map-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TempoMap(TempoMapTool::Flatten { tempo: 0 }),
        )
        .expect_err("invalid tempo should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
