use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use crate::{
    midi::{file_processing::process_midi_files_job, MidiFileProcessingConfig, MidiFileSelection},
    protocol::{CoreErrorCode, CoreEvent, MidiProcessEvent, MidiProcessJobId, MidiProcessStatus},
};

use super::{
    core_state::{CoreState, MidiProcessJobState},
    support::error_event,
};

impl CoreState {
    pub(super) fn start_process_midi_files(
        &mut self,
        selection: MidiFileSelection,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    ) -> Vec<CoreEvent> {
        if self.midi_process_job.is_some() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "a midi processing job is already active",
            )];
        }
        if selection.inputs.is_empty() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "midi file selection is empty",
            )];
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let job_id = MidiProcessJobId(self.next_resource_id);
        self.next_resource_id += 1;
        self.active_midi_process_job_id = Some(job_id);
        self.midi_process_job = Some(MidiProcessJobState {
            cancel: Arc::clone(&cancel),
            status: MidiProcessStatus::Running {
                job_id,
                output: output.clone(),
                processed_inputs: 0,
                total_inputs: selection.inputs.len(),
                current_input: None,
            },
        });

        let core_handle = self.core_handle.clone();
        thread::spawn(move || {
            let _ =
                process_midi_files_job(&selection, &output, &config, job_id, &cancel, |event| {
                    let _ = core_handle.publish_midi_process_event(event);
                });
        });

        vec![CoreEvent::MidiProcessStatus {
            status: self.midi_process_status(),
        }]
    }

    pub(super) fn cancel_process_midi_files(&mut self) -> Vec<CoreEvent> {
        match &mut self.midi_process_job {
            Some(job) => {
                job.cancel.store(true, Ordering::SeqCst);
                if let MidiProcessStatus::Running {
                    job_id,
                    output,
                    processed_inputs,
                    total_inputs,
                    current_input,
                } = &job.status
                {
                    job.status = MidiProcessStatus::Cancelling {
                        job_id: *job_id,
                        output: output.clone(),
                        processed_inputs: *processed_inputs,
                        total_inputs: *total_inputs,
                        current_input: current_input.clone(),
                    };
                }
                vec![CoreEvent::MidiProcessStatus {
                    status: self.midi_process_status(),
                }]
            }
            None => vec![error_event(
                CoreErrorCode::InvalidCommand,
                "no active midi processing job",
            )],
        }
    }

    pub(super) fn handle_midi_process_update(&mut self, event: MidiProcessEvent) {
        match &event {
            MidiProcessEvent::ProcessStarted {
                job_id,
                output,
                total_inputs,
            } => {
                self.midi_process_job =
                    self.midi_process_job.take().map(|job| MidiProcessJobState {
                        cancel: job.cancel,
                        status: MidiProcessStatus::Running {
                            job_id: *job_id,
                            output: output.clone(),
                            processed_inputs: 0,
                            total_inputs: *total_inputs,
                            current_input: None,
                        },
                    });
            }
            MidiProcessEvent::InputProgress {
                job_id,
                processed_inputs,
                total_inputs,
                current_input,
            } => {
                if let Some(job) = &mut self.midi_process_job {
                    let output = match &job.status {
                        MidiProcessStatus::Running { output, .. }
                        | MidiProcessStatus::Cancelling { output, .. } => output.clone(),
                        MidiProcessStatus::Idle => return,
                    };
                    job.status = match &job.status {
                        MidiProcessStatus::Running { .. } => MidiProcessStatus::Running {
                            job_id: *job_id,
                            output,
                            processed_inputs: *processed_inputs,
                            total_inputs: *total_inputs,
                            current_input: current_input.clone(),
                        },
                        MidiProcessStatus::Cancelling { .. } => MidiProcessStatus::Cancelling {
                            job_id: *job_id,
                            output,
                            processed_inputs: *processed_inputs,
                            total_inputs: *total_inputs,
                            current_input: current_input.clone(),
                        },
                        MidiProcessStatus::Idle => return,
                    };
                }
            }
            MidiProcessEvent::ProcessFinished { .. }
            | MidiProcessEvent::ProcessCancelled { .. }
            | MidiProcessEvent::ProcessFailed { .. } => {
                self.midi_process_job = None;
                self.active_midi_process_job_id = None;
            }
        }

        self.broadcast(CoreEvent::MidiProcess { event });
        self.broadcast(CoreEvent::MidiProcessStatus {
            status: self.midi_process_status(),
        });
    }

    pub(super) fn midi_process_status(&self) -> MidiProcessStatus {
        self.midi_process_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(MidiProcessStatus::Idle)
    }
}
