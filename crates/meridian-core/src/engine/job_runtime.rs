use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use crate::protocol::{AudioRenderStatus, MidiProcessStatus, VideoRenderStatus};

pub(super) struct JobRuntime<Status> {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) worker: Option<JoinHandle<()>>,
    pub(super) status: Status,
}

pub(super) trait JobStatus: Clone {
    type JobId: Copy;

    fn active_job_id(&self) -> Option<Self::JobId>;

    fn into_cancelling(self) -> Self;
}

impl<Status> JobRuntime<Status> {
    pub(super) fn new(
        cancel: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
        status: Status,
    ) -> Self {
        Self {
            cancel,
            worker,
            status,
        }
    }

    pub(super) fn request_cancel(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub(super) fn take_worker(&mut self) -> Option<JoinHandle<()>> {
        self.worker.take()
    }

    pub(super) fn finish_worker(&mut self) {
        if let Some(worker) = self.take_worker() {
            let _ = worker.join();
        }
    }
}

impl<Status> JobRuntime<Status>
where
    Status: JobStatus,
{
    pub(super) fn request_cancel_and_mark(&mut self) {
        self.request_cancel();
        self.status = self.status.clone().into_cancelling();
    }

    pub(super) fn active_job_id(&self) -> Option<Status::JobId> {
        self.status.active_job_id()
    }
}

impl JobStatus for AudioRenderStatus {
    type JobId = crate::protocol::AudioRenderJobId;

    fn active_job_id(&self) -> Option<Self::JobId> {
        match self {
            AudioRenderStatus::Running { job_id, .. }
            | AudioRenderStatus::Cancelling { job_id, .. } => Some(*job_id),
            AudioRenderStatus::Idle => None,
        }
    }

    fn into_cancelling(self) -> Self {
        match self {
            AudioRenderStatus::Running {
                job_id,
                output,
                total_events,
                event_index,
                time_seconds,
                rendered_seconds,
                frames_written,
            }
            | AudioRenderStatus::Cancelling {
                job_id,
                output,
                total_events,
                event_index,
                time_seconds,
                rendered_seconds,
                frames_written,
            } => AudioRenderStatus::Cancelling {
                job_id,
                output,
                total_events,
                event_index,
                time_seconds,
                rendered_seconds,
                frames_written,
            },
            AudioRenderStatus::Idle => AudioRenderStatus::Idle,
        }
    }
}

impl JobStatus for VideoRenderStatus {
    type JobId = crate::protocol::VideoRenderJobId;

    fn active_job_id(&self) -> Option<Self::JobId> {
        match self {
            VideoRenderStatus::Running { job_id, .. }
            | VideoRenderStatus::Cancelling { job_id, .. } => Some(*job_id),
            VideoRenderStatus::Idle => None,
        }
    }

    fn into_cancelling(self) -> Self {
        match self {
            VideoRenderStatus::Running {
                job_id,
                output,
                container,
                fps,
                width,
                height,
                total_frames,
                frame_index,
                current_time,
                elapsed_seconds,
                audio_progress,
            }
            | VideoRenderStatus::Cancelling {
                job_id,
                output,
                container,
                fps,
                width,
                height,
                total_frames,
                frame_index,
                current_time,
                elapsed_seconds,
                audio_progress,
            } => VideoRenderStatus::Cancelling {
                job_id,
                output,
                container,
                fps,
                width,
                height,
                total_frames,
                frame_index,
                current_time,
                elapsed_seconds,
                audio_progress,
            },
            VideoRenderStatus::Idle => VideoRenderStatus::Idle,
        }
    }
}

impl JobStatus for MidiProcessStatus {
    type JobId = crate::protocol::MidiProcessJobId;

    fn active_job_id(&self) -> Option<Self::JobId> {
        match self {
            MidiProcessStatus::Running { job_id, .. }
            | MidiProcessStatus::Cancelling { job_id, .. } => Some(*job_id),
            MidiProcessStatus::Idle => None,
        }
    }

    fn into_cancelling(self) -> Self {
        match self {
            MidiProcessStatus::Running {
                job_id,
                input,
                output,
            }
            | MidiProcessStatus::Cancelling {
                job_id,
                input,
                output,
            } => MidiProcessStatus::Cancelling {
                job_id,
                input,
                output,
            },
            MidiProcessStatus::Idle => MidiProcessStatus::Idle,
        }
    }
}

pub(super) type AudioRenderJobState = JobRuntime<AudioRenderStatus>;
pub(super) type RenderJobState = JobRuntime<VideoRenderStatus>;
pub(super) type MidiProcessJobState = JobRuntime<MidiProcessStatus>;
