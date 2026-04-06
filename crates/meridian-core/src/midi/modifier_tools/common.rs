use std::fmt::Debug;

use midi_toolkit::{
    events::Event,
    io::{MIDIWriteError, MIDIWriter},
    sequence::event::Delta,
};

use crate::error::MeridianError;

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
