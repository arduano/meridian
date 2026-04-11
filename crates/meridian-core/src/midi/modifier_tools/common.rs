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

pub(super) struct ToolProgress<'a> {
    on_progress: &'a mut dyn FnMut(u8, &str),
    should_cancel: &'a dyn Fn() -> bool,
    last_progress_percent: Option<u8>,
    last_label: Option<String>,
}

impl<'a> ToolProgress<'a> {
    pub(super) fn new(
        on_progress: &'a mut dyn FnMut(u8, &str),
        should_cancel: &'a dyn Fn() -> bool,
    ) -> Self {
        Self {
            on_progress,
            should_cancel,
            last_progress_percent: None,
            last_label: None,
        }
    }

    pub(super) fn report(
        &mut self,
        progress_percent: u8,
        label: &str,
    ) -> Result<(), MeridianError> {
        ensure_not_cancelled(self.should_cancel)?;
        let progress_percent = progress_percent.min(100);
        if self.last_progress_percent == Some(progress_percent)
            && self.last_label.as_deref() == Some(label)
        {
            return Ok(());
        }

        self.last_progress_percent = Some(progress_percent);
        self.last_label = Some(label.to_owned());
        (self.on_progress)(progress_percent, label);
        Ok(())
    }

    pub(super) fn report_steps_completed(
        &mut self,
        completed: usize,
        total: usize,
        label: &str,
    ) -> Result<(), MeridianError> {
        self.report(progress_percent(completed, total), label)
    }
}

pub(super) fn ensure_not_cancelled(should_cancel: &dyn Fn() -> bool) -> Result<(), MeridianError> {
    if should_cancel() {
        Err(MeridianError::Cancelled("midi processing cancelled".into()))
    } else {
        Ok(())
    }
}

pub(super) fn progress_percent(completed: usize, total: usize) -> u8 {
    if total == 0 {
        return 100;
    }

    ((completed.saturating_mul(100)) as f32 / total as f32)
        .round()
        .clamp(0.0, 100.0) as u8
}

pub(super) fn scale_progress_range(start_percent: u8, end_percent: u8, progress_percent: u8) -> u8 {
    let span = end_percent.saturating_sub(start_percent) as u16;
    let scaled = start_percent as u16 + (span * progress_percent.min(100) as u16 + 50) / 100;
    scaled.min(100) as u8
}

pub(super) fn map_progress_range(
    start_percent: u8,
    end_percent: u8,
    completed: usize,
    total: usize,
) -> u8 {
    scale_progress_range(
        start_percent,
        end_percent,
        progress_percent(completed, total),
    )
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
