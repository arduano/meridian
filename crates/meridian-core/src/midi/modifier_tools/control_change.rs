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

const MIDI_CONTROLLER_VALUES: usize = 128;
const MIDI_CHANNEL_COUNT: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct ControlChangeTool {
    pub strip_controllers: Vec<u8>,
    pub remap_controllers: Vec<ControllerMapEntry>,
    pub scale_controllers: Vec<ControllerScaleEntry>,
    pub inject_start: Vec<ControlValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ControllerMapEntry {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ControllerScaleEntry {
    pub controller: u8,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ControlValue {
    pub channel: u8,
    pub controller: u8,
    pub value: u8,
}

struct ControlChangePlan {
    strip_lookup: [bool; MIDI_CONTROLLER_VALUES],
    remap_lookup: [u8; MIDI_CONTROLLER_VALUES],
    value_lookup: [[u8; MIDI_CONTROLLER_VALUES]; MIDI_CONTROLLER_VALUES],
    startup_controls: Vec<Delta<u64, Event>>,
}

pub(super) fn apply_control_change_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &ControlChangeTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Rewriting control changes";
    progress.report(0, label)?;
    let plan = build_control_change_plan(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed
        .midi()
        .track_count()
        .max((!plan.startup_controls.is_empty()) as usize);

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let source_track_exists = track_index < parsed.midi().track_count();

        if track_index == 0 && !plan.startup_controls.is_empty() {
            let startup = plan.startup_controls.iter().cloned().map(Ok);
            if source_track_exists {
                let body = control_changed_track_events(parsed, 0, &plan)
                    .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, startup.chain(body))?;
            } else {
                write_try_track_events(&writer, startup)?;
            }
            continue;
        }

        let body = control_changed_track_events(parsed, track_index as u32, &plan)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, body)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn control_changed_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    plan: &'a ControlChangePlan,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| map_control_change_event(event, plan)))
}

fn map_control_change_event(mut event: Event, plan: &ControlChangePlan) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Event::ControlChange(control) = &mut event else {
        return Some(event);
    };

    let controller = plan.remap_lookup[control.controller as usize];
    if plan.strip_lookup[controller as usize] {
        return None;
    }

    control.controller = controller;
    control.value = plan.value_lookup[controller as usize][control.value as usize];
    Some(event)
}

fn build_control_change_plan(tool: &ControlChangeTool) -> Result<ControlChangePlan, MeridianError> {
    let strip_lookup = build_strip_lookup(&tool.strip_controllers)?;
    let remap_lookup = build_controller_remap_lookup(&tool.remap_controllers)?;
    let value_lookup = build_value_lookup(&tool.scale_controllers)?;
    let startup_controls = build_startup_controls(&tool.inject_start)?;

    Ok(ControlChangePlan {
        strip_lookup,
        remap_lookup,
        value_lookup,
        startup_controls,
    })
}

fn build_strip_lookup(
    strip_controllers: &[u8],
) -> Result<[bool; MIDI_CONTROLLER_VALUES], MeridianError> {
    let mut lookup = [false; MIDI_CONTROLLER_VALUES];
    for &controller in strip_controllers {
        validate_controller("control_change strip controller", controller)?;
        lookup[controller as usize] = true;
    }
    Ok(lookup)
}

fn build_controller_remap_lookup(
    mappings: &[ControllerMapEntry],
) -> Result<[u8; MIDI_CONTROLLER_VALUES], MeridianError> {
    let mut lookup = std::array::from_fn(|controller| controller as u8);
    let mut assigned = [false; MIDI_CONTROLLER_VALUES];

    for mapping in mappings {
        validate_controller("control_change mapping source controller", mapping.from)?;
        validate_controller("control_change mapping target controller", mapping.to)?;

        if assigned[mapping.from as usize] {
            return Err(MeridianError::Validation(format!(
                "control_change contains multiple mappings for controller {}",
                mapping.from
            )));
        }

        assigned[mapping.from as usize] = true;
        lookup[mapping.from as usize] = mapping.to;
    }

    Ok(lookup)
}

fn build_value_lookup(
    scales: &[ControllerScaleEntry],
) -> Result<[[u8; MIDI_CONTROLLER_VALUES]; MIDI_CONTROLLER_VALUES], MeridianError> {
    let mut lookup = std::array::from_fn(|_| std::array::from_fn(|value| value as u8));
    let mut assigned = [false; MIDI_CONTROLLER_VALUES];

    for scale in scales {
        validate_controller("control_change scale controller", scale.controller)?;
        if !scale.scale.is_finite() {
            return Err(MeridianError::Validation(format!(
                "control_change scale for controller {} must be finite",
                scale.controller
            )));
        }
        if assigned[scale.controller as usize] {
            return Err(MeridianError::Validation(format!(
                "control_change contains multiple scales for controller {}",
                scale.controller
            )));
        }

        assigned[scale.controller as usize] = true;
        lookup[scale.controller as usize] =
            std::array::from_fn(|value| scale_controller_value(value as u8, scale.scale));
    }

    Ok(lookup)
}

fn build_startup_controls(
    values: &[ControlValue],
) -> Result<Vec<Delta<u64, Event>>, MeridianError> {
    let mut assigned = [[false; MIDI_CONTROLLER_VALUES]; MIDI_CHANNEL_COUNT];
    let mut startup_controls = Vec::with_capacity(values.len());

    for value in values {
        validate_channel("control_change startup channel", value.channel)?;
        validate_controller("control_change startup controller", value.controller)?;
        if value.value > 127 {
            return Err(MeridianError::Validation(format!(
                "control_change startup value {} is out of range 0..127",
                value.value
            )));
        }
        if assigned[value.channel as usize][value.controller as usize] {
            return Err(MeridianError::Validation(format!(
                "control_change contains multiple startup values for channel {} controller {}",
                value.channel, value.controller
            )));
        }

        assigned[value.channel as usize][value.controller as usize] = true;
        startup_controls.push(Event::new_delta_control_change_event(
            0,
            value.channel,
            value.controller,
            value.value,
        ));
    }

    Ok(startup_controls)
}

fn scale_controller_value(value: u8, scale: f32) -> u8 {
    ((value as f32 * scale).round() as i32).clamp(0, 127) as u8
}

fn validate_channel(name: &str, channel: u8) -> Result<(), MeridianError> {
    if channel <= 15 {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "{name} {channel} is out of range 0..15"
        )))
    }
}

fn validate_controller(name: &str, controller: u8) -> Result<(), MeridianError> {
    if controller <= 127 {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "{name} {controller} is out of range 0..127"
        )))
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::{ControlChangeTool, ControlValue, ControllerMapEntry, ControllerScaleEntry};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn control_change_pass_strips_remaps_scales_and_injects() {
        let dir = TestDir::new("modifier-control-change");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(0, 0, 60, 100),
                Event::new_delta_control_change_event(4, 0, 1, 100),
                Event::new_delta_control_change_event(6, 0, 64, 127),
                note_off(8, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ControlChange(ControlChangeTool {
                strip_controllers: vec![64],
                remap_controllers: vec![ControllerMapEntry { from: 1, to: 7 }],
                scale_controllers: vec![ControllerScaleEntry {
                    controller: 7,
                    scale: 0.5,
                }],
                inject_start: vec![ControlValue {
                    channel: 0,
                    controller: 10,
                    value: 64,
                }],
            }),
        )
        .expect("control change tool should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::ControlChange(startup),
                },
                Delta {
                    delta: 0,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 4,
                    event: Event::ControlChange(remapped),
                },
                Delta {
                    delta: 14,
                    event: Event::NoteOff(_),
                },
            ] if startup.controller == 10
                && startup.value == 64
                && remapped.controller == 7
                && remapped.value == 50
        ));
    }

    #[test]
    fn control_change_rejects_duplicate_startup_values() {
        let dir = TestDir::new("modifier-control-change-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ControlChange(ControlChangeTool {
                strip_controllers: Vec::new(),
                remap_controllers: Vec::new(),
                scale_controllers: Vec::new(),
                inject_start: vec![
                    ControlValue {
                        channel: 0,
                        controller: 1,
                        value: 32,
                    },
                    ControlValue {
                        channel: 0,
                        controller: 1,
                        value: 64,
                    },
                ],
            }),
        )
        .expect_err("duplicate startup values should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
