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

const MIDI_PITCH_BEND_MIN: i16 = -8192;
const MIDI_PITCH_BEND_MAX: i16 = 8191;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(default)]
pub struct PitchBendTool {
    pub strip: bool,
    pub scale: f32,
    pub offset: i16,
    pub min_bend: i16,
    pub max_bend: i16,
}

impl Default for PitchBendTool {
    fn default() -> Self {
        Self {
            strip: false,
            scale: 1.0,
            offset: 0,
            min_bend: MIDI_PITCH_BEND_MIN,
            max_bend: MIDI_PITCH_BEND_MAX,
        }
    }
}

pub(super) fn apply_pitch_bend_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &PitchBendTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Rewriting pitch bend";
    progress.report(0, label)?;
    validate_pitch_bend_tool(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed.midi().track_count();

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let iter = pitch_bend_track_events(parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn pitch_bend_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a PitchBendTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    Some(events.filter_map_events(move |event| map_pitch_bend_event(event, tool)))
}

fn map_pitch_bend_event(mut event: Event, tool: &PitchBendTool) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Event::PitchWheelChange(pitch_bend) = &mut event else {
        return Some(event);
    };

    if tool.strip {
        return None;
    }

    pitch_bend.pitch = map_pitch_bend_value(pitch_bend.pitch, tool);
    Some(event)
}

fn validate_pitch_bend_tool(tool: &PitchBendTool) -> Result<(), MeridianError> {
    if !tool.scale.is_finite() {
        return Err(MeridianError::Validation(
            "pitch_bend scale must be a finite number".to_owned(),
        ));
    }
    if !(MIDI_PITCH_BEND_MIN..=MIDI_PITCH_BEND_MAX).contains(&tool.min_bend) {
        return Err(MeridianError::Validation(format!(
            "pitch_bend min_bend {} is out of range {}..{}",
            tool.min_bend, MIDI_PITCH_BEND_MIN, MIDI_PITCH_BEND_MAX
        )));
    }
    if !(MIDI_PITCH_BEND_MIN..=MIDI_PITCH_BEND_MAX).contains(&tool.max_bend) {
        return Err(MeridianError::Validation(format!(
            "pitch_bend max_bend {} is out of range {}..{}",
            tool.max_bend, MIDI_PITCH_BEND_MIN, MIDI_PITCH_BEND_MAX
        )));
    }
    if tool.min_bend > tool.max_bend {
        return Err(MeridianError::Validation(format!(
            "pitch_bend min_bend {} must be less than or equal to max_bend {}",
            tool.min_bend, tool.max_bend
        )));
    }

    Ok(())
}

fn map_pitch_bend_value(input: i16, tool: &PitchBendTool) -> i16 {
    let scaled = ((input as f32) * tool.scale).round() as i32 + tool.offset as i32;
    scaled.clamp(tool.min_bend as i32, tool.max_bend as i32) as i16
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::PitchBendTool;
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn pitch_bend_pass_scales_offsets_and_clamps() {
        let dir = TestDir::new("modifier-pitch-bend");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(0, 0, 60, 100),
                Event::new_delta_pitch_wheel_change_event(3, 0, 1000),
                note_off(7, 0, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::PitchBend(PitchBendTool {
                strip: false,
                scale: 2.0,
                offset: -500,
                min_bend: -2000,
                max_bend: 1200,
            }),
        )
        .expect("pitch bend tool should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 3,
                    event: Event::PitchWheelChange(pitch_bend),
                },
                Delta {
                    delta: 7,
                    event: Event::NoteOff(_),
                },
            ] if pitch_bend.pitch == 1200
        ));
    }

    #[test]
    fn pitch_bend_rejects_invalid_clamp_range() {
        let dir = TestDir::new("modifier-pitch-bend-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::PitchBend(PitchBendTool {
                strip: false,
                scale: 1.0,
                offset: 0,
                min_bend: 100,
                max_bend: 50,
            }),
        )
        .expect_err("invalid pitch bend range should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
