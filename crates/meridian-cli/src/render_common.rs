use std::{
    io::{BufWriter, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use meridian_core::{MeridianError, protocol::ProtocolEvent};

pub(super) fn install_cancel_handler() -> Result<Arc<AtomicBool>, MeridianError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel = Arc::clone(&cancel);
        ctrlc::set_handler(move || {
            cancel.store(true, Ordering::SeqCst);
        })
        .map_err(|e| MeridianError::Platform(format!("failed to install ctrl-c handler: {e}")))?;
    }
    Ok(cancel)
}

pub(super) fn write_events(
    stdout: &mut BufWriter<impl Write>,
    events: &[ProtocolEvent],
    context: &str,
) -> Result<(), MeridianError> {
    for event in events {
        write_event(stdout, event, context)?;
    }
    Ok(())
}

pub(super) fn write_event(
    stdout: &mut BufWriter<impl Write>,
    event: &ProtocolEvent,
    context: &str,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, event).map_err(|e| {
        MeridianError::Protocol(format!("failed to serialize {context}: {e}"))
    })?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

pub(super) fn assert_no_protocol_error(events: &[ProtocolEvent]) -> Result<(), MeridianError> {
    if let Some(ProtocolEvent::Error { code, message }) = events
        .iter()
        .find(|event| matches!(event, ProtocolEvent::Error { .. }))
    {
        return Err(MeridianError::Protocol(format!("{code:?}: {message}")));
    }
    Ok(())
}
