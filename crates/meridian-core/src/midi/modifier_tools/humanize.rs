use std::path::Path;

use midi_toolkit::{
    events::Event,
    notes::Note,
    prelude::{EventSequenceExt, NoteSequenceExt},
    sequence::event::{Delta, merge_events},
};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct HumanizeTool {
    pub start_jitter: i64,
    pub length_jitter: i64,
    pub velocity_jitter: i16,
    pub seed: u64,
    pub collision_mode: HumanizeCollisionMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum HumanizeCollisionMode {
    #[default]
    Stable,
    DistinguishByTrack,
    DistinguishByNote,
}

pub(super) fn apply_humanize_tool_to_parsed_file(
    parsed: &ParsedMidiFile,
    output: &Path,
    tool: &HumanizeTool,
    progress: &mut ToolProgress<'_>,
) -> Result<(), MeridianError> {
    let label = "Humanizing notes";
    progress.report(0, label)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;
    let track_count = parsed.midi().track_count();

    for track_index in 0..track_count {
        progress.report_steps_completed(track_index, track_count, label)?;
        let iter = humanized_track_events(parsed, track_index as u32, tool)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    progress.report(100, label)?;
    finish_midi_writer(writer)
}

fn humanized_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a HumanizeTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let note_events = humanized_note_events(parsed, track_index, tool)?;
    let non_note_events = non_note_track_events(parsed, track_index)?;
    Some(merge_events(note_events, non_note_events))
}

fn humanized_note_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    tool: &'a HumanizeTool,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;
    let mut previous_start = 0_u64;
    let mut note_order = 0_u64;

    Some(
        events
            .filter_note_events()
            .events_to_notes()
            .map(move |note| {
                let humanized = humanize_note(note?, tool, track_index, note_order, previous_start);
                note_order = note_order.saturating_add(1);
                if let Ok(note) = &humanized {
                    previous_start = note.start;
                }
                humanized
            })
            .notes_to_events(),
    )
}

fn non_note_track_events(
    parsed: &crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let events = track_events(parsed, track_index)?;
    Some(
        events
            .filter_non_note_events()
            .filter_map_events(map_non_track_start_event),
    )
}

fn humanize_note(
    mut note: Note<u64>,
    tool: &HumanizeTool,
    track_index: u32,
    note_order: u64,
    previous_start: u64,
) -> Result<Note<u64>, MeridianError> {
    let original_start = note.start;
    let original_len = note.len;
    let start_jitter = deterministic_jitter(
        tool.start_jitter,
        tool.seed,
        track_index,
        &note,
        note_order,
        0x13a5_b7cd_u64,
        tool.collision_mode,
    );
    let length_jitter = deterministic_jitter(
        tool.length_jitter,
        tool.seed,
        track_index,
        &note,
        note_order,
        0x91d8_e401_u64,
        tool.collision_mode,
    );
    let velocity_jitter = deterministic_jitter(
        tool.velocity_jitter as i64,
        tool.seed,
        track_index,
        &note,
        note_order,
        0x4c2f_9a73_u64,
        tool.collision_mode,
    ) as i16;

    note.start = apply_signed(original_start, start_jitter).max(previous_start);
    note.len = apply_signed(original_len, length_jitter).max(1);
    note.velocity = ((note.velocity as i16 + velocity_jitter).clamp(1, 127)) as u8;
    Ok(note)
}

fn deterministic_jitter(
    amount: i64,
    seed: u64,
    track_index: u32,
    note: &Note<u64>,
    note_order: u64,
    salt: u64,
    collision_mode: HumanizeCollisionMode,
) -> i64 {
    if amount == 0 {
        return 0;
    }

    let amount = amount.unsigned_abs();
    let mut hash = splitmix64(seed ^ salt);
    hash = splitmix64(hash ^ note.start);
    hash = splitmix64(hash ^ ((note.channel as u64) << 8));
    hash = splitmix64(hash ^ ((note.key as u64) << 16));

    if matches!(
        collision_mode,
        HumanizeCollisionMode::DistinguishByTrack | HumanizeCollisionMode::DistinguishByNote
    ) {
        hash = splitmix64(hash ^ track_index as u64);
    }

    if matches!(collision_mode, HumanizeCollisionMode::DistinguishByNote) {
        hash = splitmix64(hash ^ note_order);
    }

    let span = amount.saturating_mul(2).saturating_add(1);
    (hash % span) as i64 - amount as i64
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn apply_signed(value: u64, offset: i64) -> u64 {
    if offset >= 0 {
        value.saturating_add(offset as u64)
    } else {
        value.saturating_sub(offset.unsigned_abs())
    }
}

fn map_non_track_start_event(event: Event) -> Option<Event> {
    match event {
        Event::TrackStart(_) => None,
        event => Some(event),
    }
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::Event;

    use super::{HumanizeCollisionMode, HumanizeTool, deterministic_jitter};
    use crate::midi::{
        MidiModifierTool,
        parsed::ParsedMidiFile,
        test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
    };

    #[test]
    fn humanize_stable_mode_keeps_colliding_notes_consistent() {
        let dir = TestDir::new("modifier-humanize-stable");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(10, 0, 60, 100),
                note_on(0, 0, 60, 100),
                note_off(10, 0, 60),
                note_off(0, 0, 60),
            ]],
        );

        let tool = HumanizeTool {
            start_jitter: 0,
            length_jitter: 0,
            velocity_jitter: 20,
            seed: 7,
            collision_mode: HumanizeCollisionMode::Stable,
        };

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Humanize(tool.clone()),
        )
        .expect("humanize should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let velocities = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| match event.event {
                Event::NoteOn(note) => Some(note.velocity),
                _ => None,
            })
            .collect::<Vec<_>>();

        let jitter = deterministic_jitter(
            tool.velocity_jitter as i64,
            tool.seed,
            0,
            &midi_toolkit::notes::Note {
                start: 10,
                len: 10,
                key: 60,
                channel: 0,
                velocity: 100,
            },
            0,
            0x4c2f_9a73_u64,
            tool.collision_mode,
        ) as i16;

        assert_eq!(velocities, vec![((100 + jitter).clamp(1, 127)) as u8; 2]);
    }

    #[test]
    fn humanize_can_distinguish_colliding_notes_by_track() {
        let tool = HumanizeTool {
            start_jitter: 0,
            length_jitter: 0,
            velocity_jitter: 32,
            seed: 123,
            collision_mode: HumanizeCollisionMode::DistinguishByTrack,
        };

        let note = midi_toolkit::notes::Note {
            start: 10,
            len: 10,
            key: 60,
            channel: 0,
            velocity: 100,
        };

        let first = deterministic_jitter(
            tool.velocity_jitter as i64,
            tool.seed,
            0,
            &note,
            0,
            0x4c2f_9a73_u64,
            tool.collision_mode,
        );
        let second = deterministic_jitter(
            tool.velocity_jitter as i64,
            tool.seed,
            1,
            &note,
            0,
            0x4c2f_9a73_u64,
            tool.collision_mode,
        );

        assert_ne!(first, second);
    }

    #[test]
    fn humanize_can_distinguish_colliding_notes_by_note_order() {
        let tool = HumanizeTool {
            start_jitter: 0,
            length_jitter: 0,
            velocity_jitter: 32,
            seed: 123,
            collision_mode: HumanizeCollisionMode::DistinguishByNote,
        };

        let note = midi_toolkit::notes::Note {
            start: 10,
            len: 10,
            key: 60,
            channel: 0,
            velocity: 100,
        };

        let first = deterministic_jitter(
            tool.velocity_jitter as i64,
            tool.seed,
            0,
            &note,
            0,
            0x4c2f_9a73_u64,
            tool.collision_mode,
        );
        let second = deterministic_jitter(
            tool.velocity_jitter as i64,
            tool.seed,
            0,
            &note,
            1,
            0x4c2f_9a73_u64,
            tool.collision_mode,
        );

        assert_ne!(first, second);
    }

    #[test]
    fn humanize_keeps_note_starts_in_non_decreasing_order() {
        let dir = TestDir::new("modifier-humanize-order");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                note_on(10, 0, 60, 100),
                note_off(10, 0, 60),
                note_on(1, 0, 64, 100),
                note_off(10, 0, 64),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::Humanize(HumanizeTool {
                start_jitter: 20,
                length_jitter: 0,
                velocity_jitter: 0,
                seed: 99,
                collision_mode: HumanizeCollisionMode::Stable,
            }),
        )
        .expect("humanize should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let mut absolute_tick = 0_u64;
        let note_on_ticks = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| {
                absolute_tick = absolute_tick.saturating_add(event.delta);
                matches!(event.event, Event::NoteOn(_)).then_some(absolute_tick)
            })
            .collect::<Vec<_>>();

        assert!(
            note_on_ticks
                .windows(2)
                .all(|window| window[0] <= window[1])
        );
    }
}
