use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    error::MeridianError,
    protocol::{MidiProcessEvent, MidiProcessJobId},
};

use super::{
    modifier_tools::apply_modifier_tool_to_file, parsed::ParsedMidiFile,
    processing::MidiFileProcessingConfig,
};

#[derive(Debug, Clone)]
pub struct MidiFileProcessSummary {
    pub input: PathBuf,
    pub output: PathBuf,
    pub output_track_count: usize,
    pub output_ppq: u16,
    pub total_events: usize,
}

pub fn process_midi_file_to_file(
    input: &Path,
    output: &Path,
    config: &MidiFileProcessingConfig,
) -> Result<MidiFileProcessSummary, MeridianError> {
    process_midi_file_to_file_inner(input, output, config, &AtomicBool::new(false))
}

pub fn process_midi_file_job(
    input: &Path,
    output: &Path,
    config: &MidiFileProcessingConfig,
    job_id: MidiProcessJobId,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(MidiProcessEvent),
) -> Result<(), MeridianError> {
    on_event(MidiProcessEvent::ProcessStarted {
        job_id,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
    });
    match process_midi_file_to_file_inner(input, output, config, cancel) {
        Ok(summary) => {
            if cancel.load(Ordering::SeqCst) {
                on_event(MidiProcessEvent::ProcessCancelled {
                    job_id,
                    input: summary.input,
                    output: output.to_path_buf(),
                });
            } else {
                on_event(MidiProcessEvent::ProcessFinished {
                    job_id,
                    input: summary.input,
                    output: summary.output,
                    output_track_count: summary.output_track_count,
                    output_ppq: summary.output_ppq,
                    total_events: summary.total_events,
                });
            }
            Ok(())
        }
        Err(error) => {
            if cancel.load(Ordering::SeqCst) {
                on_event(MidiProcessEvent::ProcessCancelled {
                    job_id,
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                });
                Ok(())
            } else {
                on_event(MidiProcessEvent::ProcessFailed {
                    job_id,
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: error.to_string(),
                });
                Err(error)
            }
        }
    }
}

fn process_midi_file_to_file_inner(
    input: &Path,
    output: &Path,
    config: &MidiFileProcessingConfig,
    cancel: &AtomicBool,
) -> Result<MidiFileProcessSummary, MeridianError> {
    if cancel.load(Ordering::SeqCst) {
        return Ok(MidiFileProcessSummary {
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            output_track_count: 0,
            output_ppq: 0,
            total_events: 0,
        });
    }

    apply_modifier_tool_to_file(input, output, &config.tool)?;

    let written = ParsedMidiFile::load_from_file(output.to_path_buf())?;
    Ok(MidiFileProcessSummary {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        output_track_count: written.midi().track_count(),
        output_ppq: written.midi().ppq(),
        total_events: written.total_event_count()? as usize,
    })
}
