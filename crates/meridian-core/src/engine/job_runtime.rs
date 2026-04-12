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
}

pub(super) type AudioRenderJobState = JobRuntime<AudioRenderStatus>;
pub(super) type RenderJobState = JobRuntime<VideoRenderStatus>;
pub(super) type MidiProcessJobState = JobRuntime<MidiProcessStatus>;
