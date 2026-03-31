mod analysis_job;
mod audio_render_job;
mod core_state;
mod midi_process_job;
mod resource_types;
mod resources;
mod state_ops;
mod support;
mod video_render_job;

use std::{
    sync::{Arc, Mutex},
    thread,
};

use flume::{Receiver, Sender};

use crate::{
    audio::AudioRenderEvent,
    error::MeridianError,
    protocol::{
        CoreCommand, CoreEvent, MidiAnalysisJobEvent, MidiProcessEvent, RenderedFrame,
        VideoRenderEvent,
    },
};

pub use support::{error_code, event_to_error};

#[derive(Debug)]
enum RequestMessage {
    Command {
        command: CoreCommand,
        reply: Sender<CoreResponse>,
    },
    RenderFrame {
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
        reply: Sender<Result<RenderedFrame, MeridianError>>,
    },
    VideoRenderUpdate {
        event: VideoRenderEvent,
    },
    AudioRenderUpdate {
        event: AudioRenderEvent,
    },
    MidiProcessUpdate {
        event: MidiProcessEvent,
    },
    AnalysisJobUpdate {
        event: MidiAnalysisJobEvent,
    },
}

pub type CoreResponse = Vec<CoreEvent>;

#[derive(Clone)]
pub struct CoreHandle {
    sender: Sender<RequestMessage>,
    subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
}

pub fn spawn_core() -> CoreHandle {
    let (sender, receiver) = flume::unbounded();
    let subscribers = Arc::new(Mutex::new(Vec::new()));
    let core_handle = CoreHandle {
        sender: sender.clone(),
        subscribers: Arc::clone(&subscribers),
    };
    thread::spawn(move || {
        let mut core = core_state::CoreState::new(sender, subscribers);
        core.run(receiver);
    });
    core_handle
}

impl CoreHandle {
    pub fn request(&self, command: CoreCommand) -> Result<CoreResponse, MeridianError> {
        let (reply_tx, reply_rx) = flume::bounded(1);
        self.sender
            .send(RequestMessage::Command {
                command,
                reply: reply_tx,
            })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Wgpu("core reply channel closed".into()))
    }

    pub fn render_frame(
        &self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<RenderedFrame, MeridianError> {
        let (reply_tx, reply_rx) = flume::bounded(1);
        self.sender
            .send(RequestMessage::RenderFrame {
                viewport_width,
                viewport_height,
                reply: reply_tx,
            })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Wgpu("core reply channel closed".into()))?
    }

    pub fn subscribe_events(&self) -> Receiver<CoreEvent> {
        let (sender, receiver) = flume::unbounded();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(sender);
        }
        receiver
    }

    pub(crate) fn publish_video_event(&self, event: VideoRenderEvent) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::VideoRenderUpdate { event })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))
    }

    pub(crate) fn publish_audio_event(&self, event: AudioRenderEvent) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::AudioRenderUpdate { event })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))
    }

    pub(crate) fn publish_midi_process_event(
        &self,
        event: MidiProcessEvent,
    ) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::MidiProcessUpdate { event })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))
    }

    pub(crate) fn publish_analysis_job_event(
        &self,
        event: MidiAnalysisJobEvent,
    ) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::AnalysisJobUpdate { event })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))
    }
}
