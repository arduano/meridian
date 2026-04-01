use std::time::Duration;

use flume::Receiver;

use crate::{CoreHandle, MeridianError, spawn_core};

use super::server::execute_protocol_request;
use super::{ProtocolCommand, ProtocolEvent, ProtocolRequest, ProtocolResponse};

pub struct ProtocolClient {
    core: CoreHandle,
    events: Receiver<crate::protocol::CoreEvent>,
    next_id: std::sync::atomic::AtomicU64,
}

impl ProtocolClient {
    pub fn spawn() -> Self {
        let core = spawn_core();
        let events = core.subscribe_events();
        Self {
            core,
            events,
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    pub fn request(&self, command: ProtocolCommand) -> Result<ProtocolResponse, MeridianError> {
        execute_protocol_request(
            &self.core,
            ProtocolRequest {
                protocol_version: super::PROTOCOL_VERSION,
                id: Some(
                    self.next_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                ),
                command,
            },
        )
    }

    pub fn recv(&self) -> Result<ProtocolEvent, MeridianError> {
        loop {
            let event = self
                .events
                .recv()
                .map_err(|_| MeridianError::Transport("protocol event channel closed".into()))?;
            if let Ok(event) = ProtocolEvent::try_from(event) {
                return Ok(event);
            }
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<ProtocolEvent, MeridianError> {
        loop {
            let event = self.events.recv_timeout(timeout).map_err(|_| {
                MeridianError::Transport("timed out waiting for protocol event".into())
            })?;
            if let Ok(event) = ProtocolEvent::try_from(event) {
                return Ok(event);
            }
        }
    }

    pub fn shutdown(&self) -> Result<(), MeridianError> {
        let _ = self.request(ProtocolCommand::Shutdown)?;
        Ok(())
    }
}
