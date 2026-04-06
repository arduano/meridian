use std::path::Path;

use midi_toolkit::{
    events::{Event, MIDIEvent},
    io::MIDIWriter,
    prelude::EventSequenceExt,
    sequence::event::Delta,
};

use crate::{
    error::MeridianError,
    midi::{
        modifier_tools::common::{
            map_toolkit_event_result, midi_write_error, write_try_track_events,
        },
        parsed::ParsedMidiFile,
        tools::ChannelRemapTool,
    },
};

pub(super) fn apply_channel_remap_tool_to_file(
    input: &Path,
    output: &Path,
    tool: &ChannelRemapTool,
) -> Result<(), MeridianError> {
    let parsed = ParsedMidiFile::load_from_file(input.to_path_buf())?;
    let channel_lookup = build_channel_lookup(tool)?;
    let writer = MIDIWriter::new(output.to_string_lossy().as_ref(), parsed.midi().ppq())
        .map_err(midi_write_error)?;

    for track_index in 0..parsed.midi().track_count() {
        let iter = remapped_track_events(&parsed, track_index as u32, channel_lookup)
            .expect("track iteration should exist for a known track index");
        write_try_track_events(&writer, iter)?;
    }

    let mut writer = writer;
    writer.end().map_err(midi_write_error)?;
    Ok(())
}

fn remapped_track_events<'a>(
    parsed: &'a ParsedMidiFile,
    track_index: u32,
    channel_lookup: [u8; 16],
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + 'a> {
    let track = parsed.midi().iter_track(track_index)?;
    let events = track.map(map_toolkit_event_result);

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
