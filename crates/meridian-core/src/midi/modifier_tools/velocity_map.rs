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

const MIDI_VELOCITY_VALUES: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VelocityMapTool {
    Scale { scale: f32 },
    Gamma { gamma: f32 },
    Polyline { points: Vec<VelocityPoint> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct VelocityPoint {
    pub input: u8,
    pub output: u8,
}

pub(super) fn apply_velocity_map_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &VelocityMapTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Remapping velocities";
    progress.report(0, label)?;
    validate_velocity_map_tool(tool)?;
    let velocity_lookup = build_velocity_lookup(tool);
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed.midi().track_count();

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let iter = velocity_mapped_track_events(parsed, track_index as u32, velocity_lookup)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn velocity_mapped_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    velocity_lookup: [u8; MIDI_VELOCITY_VALUES],
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;

    Some(events.filter_map_events(move |event| map_velocity_event(event, velocity_lookup)))
}

fn map_velocity_event(
    mut event: Event,
    velocity_lookup: [u8; MIDI_VELOCITY_VALUES],
) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    match &mut event {
        Event::NoteOn(note) => note.velocity = velocity_lookup[note.velocity as usize],
        Event::PolyphonicKeyPressure(pressure) => {
            pressure.velocity = velocity_lookup[pressure.velocity as usize];
        }
        _ => {}
    }

    Some(event)
}

fn validate_velocity_map_tool(tool: &VelocityMapTool) -> Result<(), MeridianError> {
    match tool {
        VelocityMapTool::Scale { scale } => validate_finite("velocity_map scale", *scale),
        VelocityMapTool::Gamma { gamma } => validate_finite("velocity_map gamma", *gamma),
        VelocityMapTool::Polyline { points } => {
            let mut seen_inputs = [false; MIDI_VELOCITY_VALUES];
            for point in points {
                if seen_inputs[point.input as usize] {
                    return Err(MeridianError::Validation(format!(
                        "velocity_map polyline contains multiple points for input velocity {}",
                        point.input
                    )));
                }
                seen_inputs[point.input as usize] = true;
            }
            Ok(())
        }
    }
}

fn validate_finite(name: &str, value: f32) -> Result<(), MeridianError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "{name} must be a finite number"
        )))
    }
}

fn build_velocity_lookup(tool: &VelocityMapTool) -> [u8; MIDI_VELOCITY_VALUES] {
    match tool {
        VelocityMapTool::Scale { .. } | VelocityMapTool::Gamma { .. } => {
            std::array::from_fn(|velocity| map_velocity_value(velocity as u8, tool))
        }
        VelocityMapTool::Polyline { points } => {
            let mut sorted_points = points.clone();
            sorted_points.sort_by_key(|point| point.input);
            std::array::from_fn(|velocity| {
                map_sorted_polyline_velocity(velocity as u8, &sorted_points)
            })
        }
    }
}

fn map_velocity_value(value: u8, tool: &VelocityMapTool) -> u8 {
    match tool {
        VelocityMapTool::Scale { scale } => {
            ((value as f32 * scale).round() as i32).clamp(0, 127) as u8
        }
        VelocityMapTool::Gamma { gamma } => {
            let gamma = gamma.max(0.0001);
            (((value as f32 / 127.0).powf(gamma) * 127.0).round() as i32).clamp(0, 127) as u8
        }
        VelocityMapTool::Polyline { points } => {
            let mut sorted_points = points.to_vec();
            sorted_points.sort_by_key(|point| point.input);
            map_sorted_polyline_velocity(value, &sorted_points)
        }
    }
}

fn map_sorted_polyline_velocity(value: u8, points: &[VelocityPoint]) -> u8 {
    if points.is_empty() {
        return value;
    }

    if value <= points[0].input {
        return points[0].output;
    }

    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if value <= right.input {
            let span = (right.input - left.input).max(1) as f32;
            let t = (value - left.input) as f32 / span;
            return (left.output as f32 + (right.output as f32 - left.output as f32) * t)
                .round()
                .clamp(0.0, 127.0) as u8;
        }
    }

    points.last().map(|point| point.output).unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;

    use super::{VelocityMapTool, VelocityPoint};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn velocity_map_pass_rewrites_note_and_poly_pressure_velocities() {
        let dir = TestDir::new("modifier-velocity-map");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_note_on_event(0, 0, 60, 64),
                Event::new_delta_polyphonic_key_pressure_event(5, 0, 60, 96),
                note_off(5, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::VelocityMap(VelocityMapTool::Scale { scale: 0.5 }),
        )
        .expect("velocity map should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let velocities = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| match event.event {
                Event::NoteOn(note) => Some(note.velocity),
                Event::PolyphonicKeyPressure(pressure) => Some(pressure.velocity),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(velocities, vec![32, 48]);
    }

    #[test]
    fn velocity_map_rejects_duplicate_polyline_inputs() {
        let dir = TestDir::new("modifier-velocity-map-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![Event::new_delta_note_on_event(0, 0, 60, 64)]],
        );

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::VelocityMap(VelocityMapTool::Polyline {
                points: vec![
                    VelocityPoint {
                        input: 64,
                        output: 32,
                    },
                    VelocityPoint {
                        input: 64,
                        output: 96,
                    },
                ],
            }),
        )
        .expect_err("duplicate polyline points should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
