mod core_state;
mod support;

use std::thread;

use flume::Sender;

use crate::{
    error::MeridianError,
    protocol::{CoreCommand, CoreEvent, RenderedFrame},
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
}

pub type CoreResponse = Vec<CoreEvent>;

#[derive(Clone)]
pub struct CoreHandle {
    sender: Sender<RequestMessage>,
}

pub fn spawn_core() -> CoreHandle {
    let (sender, receiver) = flume::unbounded();
    thread::spawn(move || {
        let mut core = core_state::CoreState::default();
        core.run(receiver);
    });
    CoreHandle { sender }
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
}
