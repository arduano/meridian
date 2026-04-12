use std::io::{BufRead, BufReader, Write};

use crate::{CoreHandle, MeridianError, spawn_core};

use super::{
    CoreErrorCode, CoreEvent, PROTOCOL_VERSION, ProtocolCommand, ProtocolEvent, ProtocolRequest,
    ProtocolResponse, UnsupportedProtocolEvent,
};

enum LoopEvent {
    Input(String),
    Async(CoreEvent),
    InputClosed,
    EventsClosed,
}

pub fn serve_protocol_json() -> Result<(), MeridianError> {
    let core = spawn_core();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    serve_protocol_json_io(core, BufReader::new(stdin), &mut stdout)
}

pub fn run_one_protocol_json(raw: &str) -> Result<(), MeridianError> {
    let request: ProtocolRequest = serde_json::from_str(raw)
        .map_err(|error| MeridianError::Protocol(format!("invalid protocol request: {error}")))?;
    let core = spawn_core();
    let response = execute_protocol_request(&core, request)?;
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|error| MeridianError::Protocol(error.to_string()))?
    );
    if !matches!(
        response.events.as_slice(),
        [ProtocolEvent::ShutdownComplete]
    ) {
        let _ = core.request(ProtocolCommand::Shutdown.into());
    }
    Ok(())
}

pub fn serve_protocol_json_io<R, W>(
    core: CoreHandle,
    input: R,
    stdout: &mut W,
) -> Result<(), MeridianError>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let event_rx = core.subscribe_events();
    let (line_tx, line_rx) = flume::unbounded::<String>();

    std::thread::spawn(move || {
        for line in input.lines() {
            match line {
                Ok(line) => {
                    if line_tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        match flume::Selector::new()
            .recv(&line_rx, |line| match line {
                Ok(line) => LoopEvent::Input(line),
                Err(_) => LoopEvent::InputClosed,
            })
            .recv(&event_rx, |event| match event {
                Ok(event) => LoopEvent::Async(event),
                Err(_) => LoopEvent::EventsClosed,
            })
            .wait()
        {
            LoopEvent::Input(line) => {
                if handle_input_line(&core, stdout, line)? {
                    break;
                }
            }
            LoopEvent::Async(event) => {
                if let Ok(event) = ProtocolEvent::try_from(event) {
                    write_protocol_json(
                        stdout,
                        &ProtocolResponse {
                            protocol_version: PROTOCOL_VERSION,
                            id: None,
                            events: vec![event],
                        },
                    )?;
                }
            }
            LoopEvent::InputClosed | LoopEvent::EventsClosed => break,
        }
    }

    Ok(())
}

pub fn execute_protocol_request(
    core: &CoreHandle,
    request: ProtocolRequest,
) -> Result<ProtocolResponse, MeridianError> {
    if request.protocol_version != PROTOCOL_VERSION {
        return Ok(protocol_error_response(
            request.id,
            CoreErrorCode::InvalidProtocolVersion,
            format!(
                "unsupported protocol_version {}, expected {}",
                request.protocol_version, PROTOCOL_VERSION
            ),
        ));
    }

    let response = match request.command {
        ProtocolCommand::StartRenderVideo { config } => {
            if let (Some(scene), Some(renderer)) = (&config.scene, config.renderer) {
                let expected_renderer = scene.renderer_kind();
                if renderer != expected_renderer {
                    return Ok(protocol_error_response(
                        request.id,
                        CoreErrorCode::ValidationFailed,
                        format!(
                            "video render scene config requires renderer {:?}, got {:?}",
                            expected_renderer, renderer
                        ),
                    ));
                }
            }
            core.request(ProtocolCommand::StartRenderVideo { config }.into())?
        }
        command => core.request(command.into())?,
    };
    let events = response
        .into_iter()
        .map(ProtocolEvent::try_from)
        .collect::<Result<Vec<_>, UnsupportedProtocolEvent>>()
        .map_err(|_| {
            MeridianError::Protocol("unsupported core event for protocol transport".into())
        })?;
    Ok(ProtocolResponse {
        protocol_version: PROTOCOL_VERSION,
        id: request.id,
        events,
    })
}

fn handle_input_line(
    core: &CoreHandle,
    stdout: &mut impl Write,
    line: String,
) -> Result<bool, MeridianError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    let request = match serde_json::from_str::<ProtocolRequest>(trimmed) {
        Ok(request) => request,
        Err(error) => {
            write_protocol_json(
                stdout,
                &protocol_error_response(
                    None,
                    CoreErrorCode::InvalidJson,
                    format!("invalid_json: {error}"),
                ),
            )?;
            return Ok(false);
        }
    };

    let should_shutdown = matches!(request.command, ProtocolCommand::Shutdown);
    let response = execute_protocol_request(core, request)?;
    write_protocol_json(stdout, &response)?;
    Ok(should_shutdown)
}

fn protocol_error_response(
    id: Option<u64>,
    code: CoreErrorCode,
    message: impl Into<String>,
) -> ProtocolResponse {
    ProtocolResponse {
        protocol_version: PROTOCOL_VERSION,
        id,
        events: vec![ProtocolEvent::Error {
            code,
            message: message.into(),
        }],
    }
}

fn write_protocol_json(
    stdout: &mut impl Write,
    response: &ProtocolResponse,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, response)
        .map_err(|error| MeridianError::Protocol(error.to_string()))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
