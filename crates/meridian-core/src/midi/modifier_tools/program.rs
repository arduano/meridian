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

const MIDI_CHANNEL_COUNT: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct ProgramTool {
    pub force_program: Option<u8>,
    pub strip_program_changes: bool,
    pub startup_programs: Vec<ChannelProgram>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ChannelProgram {
    pub channel: u8,
    pub program: u8,
}

pub(super) fn apply_program_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &ProgramTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Rewriting program changes";
    progress.report(0, label)?;
    validate_program_tool(tool)?;
    let startup_programs = build_startup_programs(&tool.startup_programs)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed
        .midi()
        .track_count()
        .max((!startup_programs.is_empty()) as usize);

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let source_track_exists = track_index < parsed.midi().track_count();

        if track_index == 0 && !startup_programs.is_empty() {
            let startup = startup_programs.iter().cloned().map(Ok);
            if source_track_exists {
                let body = program_track_events(&parsed, 0, tool)
                    .expect("track iteration should exist for a known track index");
                write_try_track_events(&writer, startup.chain(body))?;
            } else {
                write_try_track_events(&writer, startup)?;
            }
            continue;
        }

        let body = program_track_events(&parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, body)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn program_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a ProgramTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;

    Some(events.filter_map_events(move |event| {
        map_program_event(event, tool.force_program, tool.strip_program_changes)
    }))
}

fn map_program_event(
    mut event: Event,
    force_program: Option<u8>,
    strip_program_changes: bool,
) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    let Event::ProgramChange(program) = &mut event else {
        return Some(event);
    };

    if strip_program_changes {
        return None;
    }

    if let Some(force_program) = force_program {
        program.program = force_program;
    }

    Some(event)
}

fn validate_program_tool(tool: &ProgramTool) -> Result<(), MeridianError> {
    if let Some(program) = tool.force_program {
        validate_program_value("program force_program", program)?;
    }
    for startup in &tool.startup_programs {
        validate_channel("program startup channel", startup.channel)?;
        validate_program_value("program startup program", startup.program)?;
    }
    Ok(())
}

fn build_startup_programs(
    startup_programs: &[ChannelProgram],
) -> Result<Vec<Delta<u64, Event>>, MeridianError> {
    let mut assigned = [false; MIDI_CHANNEL_COUNT];
    let mut events = Vec::with_capacity(startup_programs.len());

    for startup in startup_programs {
        if assigned[startup.channel as usize] {
            return Err(MeridianError::Validation(format!(
                "program contains multiple startup programs for channel {}",
                startup.channel
            )));
        }

        assigned[startup.channel as usize] = true;
        events.push(Event::new_delta_program_change_event(
            0,
            startup.channel,
            startup.program,
        ));
    }

    Ok(events)
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

fn validate_program_value(name: &str, program: u8) -> Result<(), MeridianError> {
    if program <= 127 {
        Ok(())
    } else {
        Err(MeridianError::Validation(format!(
            "{name} {program} is out of range 0..127"
        )))
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;
    use midi_toolkit::sequence::event::Delta;

    use super::{ChannelProgram, ProgramTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn program_pass_strips_rewrites_and_injects_startup_programs() {
        let dir = TestDir::new("modifier-program");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_program_change_event(0, 2, 10),
                note_on(6, 2, 60, 100),
                note_off(12, 2, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Program(ProgramTool {
                force_program: Some(42),
                strip_program_changes: false,
                startup_programs: vec![ChannelProgram {
                    channel: 2,
                    program: 8,
                }],
            }),
        )
        .expect("program tool should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let events = read_track_events(&parsed, 0);

        assert!(matches!(
            events.as_slice(),
            [
                Delta {
                    delta: 0,
                    event: Event::ProgramChange(startup),
                },
                Delta {
                    delta: 0,
                    event: Event::ProgramChange(rewritten),
                },
                Delta {
                    delta: 6,
                    event: Event::NoteOn(_),
                },
                Delta {
                    delta: 12,
                    event: Event::NoteOff(_),
                },
            ] if startup.channel == 2
                && startup.program == 8
                && rewritten.channel == 2
                && rewritten.program == 42
        ));
    }

    #[test]
    fn program_rejects_duplicate_startup_channels() {
        let dir = TestDir::new("modifier-program-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Program(ProgramTool {
                force_program: None,
                strip_program_changes: false,
                startup_programs: vec![
                    ChannelProgram {
                        channel: 0,
                        program: 8,
                    },
                    ChannelProgram {
                        channel: 0,
                        program: 9,
                    },
                ],
            }),
        )
        .expect_err("duplicate startup programs should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
