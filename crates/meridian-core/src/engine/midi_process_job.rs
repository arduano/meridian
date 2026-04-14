use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    thread,
};

use crate::{
    midi::{MidiFileProcessingConfig, file_processing::process_midi_file_job},
    protocol::{CoreErrorCode, CoreEvent, MidiProcessEvent, MidiProcessJobId, MidiProcessStatus},
};

use super::{core_state::CoreState, job_runtime::MidiProcessJobState, support::error_event};

impl CoreState {
    pub(super) fn start_process_midi_file(
        &mut self,
        input: PathBuf,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    ) -> Vec<CoreEvent> {
        if self.midi_process_job.is_some() {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                "A MIDI processing job is already active",
            )];
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let job_id = MidiProcessJobId(self.resource_ids.next_job_id());
        self.midi_process_job = Some(MidiProcessJobState::new(
            Arc::clone(&cancel),
            None,
            MidiProcessStatus::Running {
                job_id,
                input: input.clone(),
                output: output.clone(),
            },
        ));

        let core_handle = self.core_handle.clone();
        thread::spawn(move || {
            let _ = process_midi_file_job(&input, &output, &config, job_id, &cancel, |event| {
                let _ = core_handle.publish_midi_process_event(event);
            });
        });

        vec![CoreEvent::MidiProcessStatus {
            status: self.midi_process_status(),
        }]
    }

    pub(super) fn cancel_midi_file_process(&mut self) -> Vec<CoreEvent> {
        match &mut self.midi_process_job {
            Some(job) => {
                job.request_cancel_and_mark();
                vec![CoreEvent::MidiProcessStatus {
                    status: self.midi_process_status(),
                }]
            }
            None => vec![error_event(
                CoreErrorCode::InvalidCommand,
                "No active MIDI processing job",
            )],
        }
    }

    pub(super) fn handle_midi_process_update(&mut self, event: MidiProcessEvent) {
        match &event {
            MidiProcessEvent::ProcessStarted {
                job_id,
                input,
                output,
            } => {
                self.midi_process_job = self.midi_process_job.take().map(|job| {
                    MidiProcessJobState::new(
                        job.cancel,
                        None,
                        MidiProcessStatus::Running {
                            job_id: *job_id,
                            input: input.clone(),
                            output: output.clone(),
                        },
                    )
                });
            }
            MidiProcessEvent::Progress { .. } => {}
            MidiProcessEvent::ProcessFinished { .. }
            | MidiProcessEvent::ProcessCancelled { .. }
            | MidiProcessEvent::ProcessFailed { .. } => {
                self.midi_process_job = None;
            }
        }

        let should_broadcast_status = !matches!(event, MidiProcessEvent::Progress { .. });
        self.broadcast(CoreEvent::MidiProcess { event });
        if should_broadcast_status {
            self.broadcast(CoreEvent::MidiProcessStatus {
                status: self.midi_process_status(),
            });
        }
    }

    pub(super) fn midi_process_status(&self) -> MidiProcessStatus {
        self.midi_process_job
            .as_ref()
            .map(|job| job.status.clone())
            .unwrap_or(MidiProcessStatus::Idle)
    }

    pub(super) fn active_midi_process_job_id_from_state(&self) -> Option<MidiProcessJobId> {
        self.midi_process_job
            .as_ref()
            .and_then(|job| job.active_job_id())
    }
}
