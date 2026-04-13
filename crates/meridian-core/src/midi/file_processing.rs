use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    error::MeridianError,
    protocol::{MidiProcessEvent, MidiProcessJobId},
};

use super::{
    cleanup_output_file, modifier_tools::apply_modifier_tool_to_parsed_file,
    parsed::ParsedMidiFile, processing::MidiFileProcessingConfig,
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
    let mut on_event = |_event: MidiProcessEvent| {};
    let mut progress = ProcessProgressEmitter::new(MidiProcessJobId(0), &mut on_event);
    let result = process_midi_file_to_file_inner(
        input,
        output,
        config,
        &AtomicBool::new(false),
        &mut progress,
    );
    if result.is_err() {
        cleanup_output_file(output);
    }
    result
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
    let result = {
        let mut progress = ProcessProgressEmitter::new(job_id, &mut on_event);
        process_midi_file_to_file_inner(input, output, config, cancel, &mut progress)
    };
    match result {
        Ok(summary) => {
            if cancel.load(Ordering::SeqCst) {
                cleanup_output_file(output);
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
            cleanup_output_file(output);
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
    progress: &mut ProcessProgressEmitter<'_>,
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

    progress.report_weighted(0, 10, 0.0, "Loading MIDI")?;
    let parsed = ParsedMidiFile::load_from_file_with_progress_cancelable(
        input.to_path_buf(),
        |value| {
            let _ = progress.report_weighted(0, 10, value, "Loading MIDI");
        },
        || cancel.load(Ordering::SeqCst),
    )?;
    progress.report_percent(10, "Preparing tool")?;

    apply_modifier_tool_to_parsed_file(
        output,
        &config.tool,
        &parsed,
        &mut |progress_percent, label| {
            let _ = progress.report_weighted(15, 95, progress_percent as f32 / 100.0, label);
        },
        &|| cancel.load(Ordering::SeqCst),
    )?;

    progress.report_percent(95, "Inspecting output")?;
    let written = ParsedMidiFile::load_from_file_with_progress_cancelable(
        output.to_path_buf(),
        |value| {
            let _ = progress.report_weighted(95, 100, value, "Inspecting output");
        },
        || cancel.load(Ordering::SeqCst),
    )?;
    Ok(MidiFileProcessSummary {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        output_track_count: written.midi().track_count(),
        output_ppq: written.midi().ppq(),
        total_events: written.total_event_count()? as usize,
    })
}

struct ProcessProgressEmitter<'a> {
    job_id: MidiProcessJobId,
    on_event: &'a mut dyn FnMut(MidiProcessEvent),
    last_progress_percent: Option<u8>,
    last_label: Option<String>,
}

impl<'a> ProcessProgressEmitter<'a> {
    fn new(job_id: MidiProcessJobId, on_event: &'a mut dyn FnMut(MidiProcessEvent)) -> Self {
        Self {
            job_id,
            on_event,
            last_progress_percent: None,
            last_label: None,
        }
    }

    fn report_weighted(
        &mut self,
        start_percent: u8,
        end_percent: u8,
        value: f32,
        label: &str,
    ) -> Result<(), MeridianError> {
        let clamped = value.clamp(0.0, 1.0);
        let progress_percent =
            start_percent as f32 + (end_percent.saturating_sub(start_percent)) as f32 * clamped;
        self.report_percent(progress_percent.round().clamp(0.0, 100.0) as u8, label)
    }

    fn report_percent(&mut self, progress_percent: u8, label: &str) -> Result<(), MeridianError> {
        let progress_percent = progress_percent.min(100);
        if self.last_progress_percent == Some(progress_percent)
            && self.last_label.as_deref() == Some(label)
        {
            return Ok(());
        }

        self.last_progress_percent = Some(progress_percent);
        self.last_label = Some(label.to_owned());
        (self.on_event)(MidiProcessEvent::Progress {
            job_id: self.job_id,
            progress_percent,
            label: label.to_owned(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::{
        error::MeridianError,
        midi::{
            MidiModifierTool,
            modifier_tools::NoteLengthTool,
            processing::MidiFileProcessingConfig,
            test_support::{TestDir, note_off, note_on, write_toolkit_midi},
        },
        protocol::MidiProcessJobId,
    };

    #[test]
    fn process_job_cleans_partial_output_on_cancel() {
        let dir = TestDir::new("process-cleanup-on-cancel");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(0, 0, 60, 100), note_off(12, 0, 60)]],
        );

        let cancel = AtomicBool::new(false);
        let mut saw_progress = false;
        let result = super::process_midi_file_job(
            &input,
            &output,
            &MidiFileProcessingConfig {
                tool: MidiModifierTool::NoteLength(NoteLengthTool {
                    min_ticks: None,
                    max_ticks: None,
                    scale: Some(1.0),
                    fixed_ticks: None,
                }),
            },
            MidiProcessJobId(1),
            &cancel,
            |event| {
                if let crate::protocol::MidiProcessEvent::Progress { .. } = event
                    && !saw_progress {
                        saw_progress = true;
                        cancel.store(true, Ordering::SeqCst);
                    }
            },
        );

        assert!(result.is_ok());
        assert!(!output.exists());
    }

    #[test]
    fn process_sync_cleans_output_on_error() {
        let dir = TestDir::new("process-cleanup-on-error");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input,
            96,
            &[vec![note_on(0, 0, 60, 100), note_off(12, 0, 60)]],
        );

        let result = super::process_midi_file_to_file(
            &input,
            &output,
            &MidiFileProcessingConfig {
                tool: MidiModifierTool::NoteLength(NoteLengthTool {
                    min_ticks: None,
                    max_ticks: None,
                    scale: Some(f32::NAN),
                    fixed_ticks: None,
                }),
            },
        );

        assert!(matches!(result, Err(MeridianError::Validation(_))));
        assert!(!output.exists());
    }
}
