use std::io::{self, BufRead, Write};

use meridian_core::{
    MeridianError,
    protocol::{
        CoreCommand, CoreErrorCode, CoreEvent, JsonRequest, JsonResponse, PROTOCOL_VERSION,
    },
    spawn_core,
};

pub fn serve_json() -> Result<(), MeridianError> {
    let core = spawn_core();
    let event_rx = core.subscribe_events();
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let (line_tx, line_rx) = flume::unbounded::<String>();

    std::thread::spawn(move || {
        for line in stdin.lock().lines() {
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
                if handle_input_line(&core, &mut stdout, line)? {
                    break;
                }
            }
            LoopEvent::Async(event) => {
                handle_async_event(&mut stdout, event)?;
            }
            LoopEvent::InputClosed | LoopEvent::EventsClosed => break,
        }
    }

    Ok(())
}

pub fn run_one_json(raw: &str) -> Result<(), MeridianError> {
    let request: JsonRequest = serde_json::from_str(raw)
        .map_err(|error| MeridianError::InvalidMidi(format!("invalid json request: {error}")))?;
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(MeridianError::InvalidMidi(format!(
            "unsupported protocol_version {}, expected {}",
            request.protocol_version, PROTOCOL_VERSION
        )));
    }

    let core = spawn_core();
    let response = JsonResponse {
        protocol_version: PROTOCOL_VERSION,
        id: request.id,
        events: core.request(request.command)?,
    };
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?
    );

    if !matches!(
        response.events.as_slice(),
        [meridian_core::protocol::CoreEvent::ShutdownComplete]
    ) {
        let _ = core.request(CoreCommand::Shutdown);
    }
    Ok(())
}

enum LoopEvent {
    Input(String),
    Async(CoreEvent),
    InputClosed,
    EventsClosed,
}

fn handle_input_line(
    core: &meridian_core::CoreHandle,
    stdout: &mut impl Write,
    line: String,
) -> Result<bool, MeridianError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    let request = match serde_json::from_str::<JsonRequest>(trimmed) {
        Ok(request) => request,
        Err(error) => {
            write_json(
                stdout,
                &JsonResponse {
                    protocol_version: PROTOCOL_VERSION,
                    id: None,
                    events: vec![CoreEvent::Error {
                        code: CoreErrorCode::InvalidJson,
                        message: format!("invalid_json: {error}"),
                    }],
                },
            )?;
            return Ok(false);
        }
    };

    if request.protocol_version != PROTOCOL_VERSION {
        write_json(
            stdout,
            &JsonResponse {
                protocol_version: PROTOCOL_VERSION,
                id: request.id,
                events: vec![CoreEvent::Error {
                    code: CoreErrorCode::InvalidProtocolVersion,
                    message: format!(
                        "unsupported protocol_version {}, expected {}",
                        request.protocol_version, PROTOCOL_VERSION
                    ),
                }],
            },
        )?;
        return Ok(false);
    }

    let should_shutdown = matches!(request.command, CoreCommand::Shutdown);
    let response = JsonResponse {
        protocol_version: PROTOCOL_VERSION,
        id: request.id,
        events: core.request(request.command)?,
    };
    write_json(stdout, &response)?;
    Ok(should_shutdown)
}

fn handle_async_event(stdout: &mut impl Write, event: CoreEvent) -> Result<(), MeridianError> {
    match event {
        CoreEvent::VideoRender { .. } | CoreEvent::VideoRenderStatus { .. } => {
            write_json(
                stdout,
                &JsonResponse {
                    protocol_version: PROTOCOL_VERSION,
                    id: None,
                    events: vec![event],
                },
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn write_json(stdout: &mut impl Write, response: &JsonResponse) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, response)
        .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
