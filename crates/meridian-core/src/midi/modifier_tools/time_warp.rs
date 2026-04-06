use std::path::Path;

use midi_toolkit::{events::Event, sequence::event::Delta};
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
pub struct TimeWarpTool {
    pub points: Vec<TimeWarpPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TimeWarpPoint {
    pub source_tick: u64,
    pub dest_tick: u64,
}

pub(super) fn apply_time_warp_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &TimeWarpTool,
) -> Result<(), MeridianError> {
    let parsed = load_parsed_midi(input)?;
    let warp_points = build_time_warp_points(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = warped_track_events(&parsed, track_index as u32, &warp_points)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    finish_midi_writer(writer)
}

fn warped_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    warp_points: &'a [TimeWarpPoint],
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    let mut source_tick = 0_u64;
    let mut warped_tick = 0_u64;

    Some(
        events
            .filter_map(map_non_track_start_result)
            .map(move |event| {
                let mut event = event?;
                source_tick = source_tick.saturating_add(event.delta);
                let next_warped_tick = map_time_warp_tick(source_tick, warp_points);
                event.delta = next_warped_tick.saturating_sub(warped_tick);
                warped_tick = next_warped_tick;
                Ok(event)
            }),
    )
}

fn map_non_track_start_result(
    event: Result<Delta<u64, Event>, MeridianError>,
) -> Option<Result<Delta<u64, Event>, MeridianError>> {
    match event {
        Ok(Delta {
            event: Event::TrackStart(_),
            ..
        }) => None,
        other => Some(other),
    }
}

fn build_time_warp_points(tool: &TimeWarpTool) -> Result<Vec<TimeWarpPoint>, MeridianError> {
    let mut points = tool.points.clone();
    points.sort_by_key(|point| point.source_tick);

    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if left.source_tick == right.source_tick {
            return Err(MeridianError::Validation(format!(
                "time_warp contains multiple points at source tick {}",
                left.source_tick
            )));
        }
        if left.dest_tick > right.dest_tick {
            return Err(MeridianError::Validation(
                "time_warp destination ticks must be monotonic to preserve event order".to_owned(),
            ));
        }
    }

    Ok(points)
}

fn map_time_warp_tick(tick: u64, points: &[TimeWarpPoint]) -> u64 {
    if points.len() < 2 {
        return tick;
    }

    if tick <= points[0].source_tick {
        return points[0].dest_tick;
    }

    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if tick <= right.source_tick {
            let span = (right.source_tick - left.source_tick).max(1) as f64;
            let t = (tick - left.source_tick) as f64 / span;
            return (left.dest_tick as f64 + (right.dest_tick as f64 - left.dest_tick as f64) * t)
                .round() as u64;
        }
    }

    points.last().map(|point| point.dest_tick).unwrap_or(tick)
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::{TimeWarpPoint, TimeWarpTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn time_warp_remaps_event_deltas_without_reordering() {
        let dir = TestDir::new("modifier-time-warp");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(10, 0, 60, 100),
                note_off(20, 0, 60),
                note_on(30, 0, 64, 110),
                note_off(40, 0, 64),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TimeWarp(TimeWarpTool {
                points: vec![
                    TimeWarpPoint {
                        source_tick: 0,
                        dest_tick: 0,
                    },
                    TimeWarpPoint {
                        source_tick: 100,
                        dest_tick: 200,
                    },
                ],
            }),
        )
        .expect("time warp should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 20,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 40,
                    event: Event::NoteOff(_),
                },
                Delta {
                    delta: 60,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 80,
                    event: Event::NoteOff(_),
                },
            ]
        ));
    }

    #[test]
    fn time_warp_rejects_non_monotonic_destination_ticks() {
        let dir = TestDir::new("modifier-time-warp-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::TimeWarp(TimeWarpTool {
                points: vec![
                    TimeWarpPoint {
                        source_tick: 0,
                        dest_tick: 100,
                    },
                    TimeWarpPoint {
                        source_tick: 10,
                        dest_tick: 50,
                    },
                ],
            }),
        )
        .expect_err("non-monotonic warp points should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
