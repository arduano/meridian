use std::time::{Duration, Instant};

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
        recv_timeout_filtered(&self.events, timeout)
    }

    pub fn shutdown(&self) -> Result<(), MeridianError> {
        let _ = self.request(ProtocolCommand::Shutdown)?;
        Ok(())
    }
}

fn recv_timeout_filtered(
    events: &Receiver<crate::protocol::CoreEvent>,
    timeout: Duration,
) -> Result<ProtocolEvent, MeridianError> {
    let deadline = Instant::now() + timeout;

    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(MeridianError::Transport(
                "timed out waiting for protocol event".into(),
            ));
        };

        let event = events
            .recv_timeout(remaining)
            .map_err(|_| MeridianError::Transport("timed out waiting for protocol event".into()))?;
        if let Ok(event) = ProtocolEvent::try_from(event) {
            return Ok(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::AtomicU64,
        time::{Duration, Instant},
    };

    use crate::{
        MeridianError,
        midi::MidiBuildProgress,
        protocol::{CoreErrorCode, CoreEvent, ProtocolEvent},
        spawn_core,
    };

    use super::ProtocolClient;

    #[test]
    fn recv_timeout_does_not_reset_after_unsupported_events() {
        let core = spawn_core();
        let (sender, receiver) = flume::unbounded();
        let client = ProtocolClient {
            core: core.clone(),
            events: receiver,
            next_id: AtomicU64::new(1),
        };

        sender
            .send(CoreEvent::MidiLoadProgress {
                path: "ignored.mid".into(),
                progress: MidiBuildProgress::from_fraction(0.25),
                status: "Loading".into(),
            })
            .expect("send unsupported event");

        let started = Instant::now();
        let error = client
            .recv_timeout(Duration::from_millis(50))
            .expect_err("unsupported-only stream should still time out");
        let elapsed = started.elapsed();

        assert!(
            elapsed >= Duration::from_millis(40),
            "timeout should wait for the remaining budget, got {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(250),
            "timeout should not restart indefinitely, got {elapsed:?}"
        );
        assert!(
            matches!(error, MeridianError::Transport(ref message) if message.contains("timed out")),
            "unexpected timeout error: {error:?}"
        );

        let _ = core.request(crate::protocol::CoreCommand::Shutdown);
    }

    #[test]
    fn recv_timeout_returns_supported_event_after_skipping_unsupported_ones() {
        let core = spawn_core();
        let (sender, receiver) = flume::unbounded();
        let client = ProtocolClient {
            core: core.clone(),
            events: receiver,
            next_id: AtomicU64::new(1),
        };

        sender
            .send(CoreEvent::MidiLoadProgress {
                path: "ignored.mid".into(),
                progress: MidiBuildProgress::from_fraction(0.5),
                status: "Loading".into(),
            })
            .expect("send unsupported event");
        sender
            .send(CoreEvent::Error {
                code: CoreErrorCode::InvalidCommand,
                message: "boom".into(),
            })
            .expect("send supported event");

        let event = client
            .recv_timeout(Duration::from_millis(50))
            .expect("supported event should be returned");
        assert!(matches!(
            event,
            ProtocolEvent::Error {
                code: CoreErrorCode::InvalidCommand,
                message,
            } if message == "boom"
        ));

        let _ = core.request(crate::protocol::CoreCommand::Shutdown);
    }
}
