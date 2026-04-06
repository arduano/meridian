use std::path::Path;

use midi_toolkit::{
    events::{Event, MIDIEvent},
    prelude::EventSequenceExt,
    sequence::event::Delta,
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
pub struct ChannelRemapTool {
    pub mappings: Vec<ChannelMapEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct ChannelMapEntry {
    pub from: u8,
    pub to: u8,
}

pub(super) fn apply_channel_remap_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &ChannelRemapTool,
) -> Result<(), MeridianError> {
    let parsed = load_parsed_midi(input)?;
    let channel_lookup = build_channel_lookup(tool)?;
    let writer = open_midi_writer(output, parsed.midi().ppq())?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = remapped_track_events(&parsed, track_index as u32, channel_lookup)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    finish_midi_writer(writer)
}

fn remapped_track_events<'a>(
    parsed: &'a crate::midi::parsed::ParsedMidiFile,
    track_index: u32,
    channel_lookup: [u8; 16],
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let events = track_events(parsed, track_index)?;

    Some(events.filter_map_events(move |event| remap_event_channel(event, channel_lookup)))
}

fn remap_event_channel(mut event: Event, channel_lookup: [u8; 16]) -> Option<Event> {
    if matches!(event, Event::TrackStart(_)) {
        return None;
    }

    if let Some(channel) = event.channel_mut() {
        *channel = channel_lookup[*channel as usize];
    }

    Some(event)
}

fn build_channel_lookup(tool: &ChannelRemapTool) -> Result<[u8; 16], MeridianError> {
    let mut lookup = std::array::from_fn(|channel| channel as u8);
    let mut assigned = [false; 16];

    for mapping in &tool.mappings {
        if mapping.from > 15 {
            return Err(MeridianError::Validation(format!(
                "channel_remap mapping source channel {} is out of range 0..15",
                mapping.from
            )));
        }
        if mapping.to > 15 {
            return Err(MeridianError::Validation(format!(
                "channel_remap mapping target channel {} is out of range 0..15",
                mapping.to
            )));
        }
        if assigned[mapping.from as usize] {
            return Err(MeridianError::Validation(format!(
                "channel_remap contains multiple mappings for source channel {}",
                mapping.from
            )));
        }

        assigned[mapping.from as usize] = true;
        lookup[mapping.from as usize] = mapping.to;
    }

    Ok(lookup)
}

#[cfg(test)]
mod tests {
    use midi_toolkit::events::{Event, MIDIEvent};

    use super::{ChannelMapEntry, ChannelRemapTool};
    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            parsed::ParsedMidiFile,
            test_support::{TestDir, note_off, note_on, read_track_events, write_toolkit_midi},
        },
    };

    #[test]
    fn channel_remap_pass_updates_channel_events() {
        let dir = TestDir::new("modifier-channel-remap");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![
                Event::new_delta_note_on_event(0, 1, 60, 100),
                Event::new_delta_control_change_event(0, 1, 64, 127),
                Event::new_delta_program_change_event(0, 1, 10),
                note_off(24, 1, 60),
            ]],
        );

        super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChannelRemap(ChannelRemapTool {
                mappings: vec![ChannelMapEntry { from: 1, to: 9 }],
            }),
        )
        .expect("channel remap should succeed");

        let parsed = ParsedMidiFile::load_from_file(output).expect("parse output midi");
        let channels = read_track_events(&parsed, 0)
            .into_iter()
            .filter_map(|event| event.event.channel())
            .collect::<Vec<_>>();

        assert_eq!(channels, vec![9, 9, 9, 9]);
    }

    #[test]
    fn channel_remap_rejects_invalid_channel_maps() {
        let dir = TestDir::new("modifier-channel-remap-invalid");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input, 96, &[vec![note_on(0, 0, 60, 100)]]);

        let error = super::super::apply_modifier_tool_to_file(
            &input,
            &output,
            &MidiModifierTool::ChannelRemap(ChannelRemapTool {
                mappings: vec![
                    ChannelMapEntry { from: 2, to: 9 },
                    ChannelMapEntry { from: 2, to: 10 },
                ],
            }),
        )
        .expect_err("duplicate channel mappings should fail");

        assert!(matches!(error, MeridianError::Validation(_)));
        assert!(!output.exists());
    }
}
