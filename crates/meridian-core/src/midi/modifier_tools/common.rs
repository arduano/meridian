use std::{fmt::Debug, path::Path};

use midi_toolkit::{
    events::Event,
    io::{MIDIWriteError, MIDIWriter},
    sequence::event::Delta,
};

use crate::{error::MeridianError, midi::parsed::ParsedMidiFile};

pub(super) fn load_parsed_midi(input: &Path) -> Result<ParsedMidiFile, MeridianError> {
    ParsedMidiFile::load_from_file(input.to_path_buf())
}

pub(super) fn open_midi_writer(output: &Path, ppq: u16) -> Result<MIDIWriter, MeridianError> {
    MIDIWriter::new(output.to_string_lossy().as_ref(), ppq).map_err(midi_write_error)
}

pub(super) fn finish_midi_writer(writer: MIDIWriter) -> Result<(), MeridianError> {
    let mut writer = writer;
    writer.end().map_err(midi_write_error)
}

pub(super) fn track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let track = parsed.midi().iter_track(track_index)?;
    Some(track.map(map_toolkit_event_result))
}

pub(super) fn map_toolkit_event_result(
    event: Result<Delta<u64, Event>, impl Debug>,
) -> Result<Delta<u64, Event>, MeridianError> {
    event.map_err(|error| MeridianError::MidiLoad(format!("{error:?}")))
}

pub(super) fn write_try_track_events(
    writer: &MIDIWriter,
    events: impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>>,
) -> Result<(), MeridianError> {
    let mut track_writer = writer.try_open_next_track().map_err(midi_write_error)?;

    // `write_events_iter` only accepts infallible items, so result-bearing streams still need a
    // small manual bridge here.
    for event in events {
        track_writer.write_event(event?).map_err(midi_write_error)?;
    }

    track_writer.end().map_err(midi_write_error)?;
    Ok(())
}

pub(super) fn midi_write_error(error: MIDIWriteError) -> MeridianError {
    MeridianError::MidiLoad(format!("midi write error: {error}"))
}
