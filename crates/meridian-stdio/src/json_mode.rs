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
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<JsonRequest>(trimmed) {
            Ok(request) => request,
            Err(error) => {
                write_json(
                    &mut stdout,
                    &JsonResponse {
                        protocol_version: PROTOCOL_VERSION,
                        id: None,
                        events: vec![CoreEvent::Error {
                            code: CoreErrorCode::InvalidJson,
                            message: format!("invalid_json: {error}"),
                        }],
                    },
                )?;
                continue;
            }
        };

        if request.protocol_version != PROTOCOL_VERSION {
            write_json(
                &mut stdout,
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
            continue;
        }

        let should_shutdown = matches!(request.command, CoreCommand::Shutdown);
        let response = JsonResponse {
            protocol_version: PROTOCOL_VERSION,
            id: request.id,
            events: core.request(request.command)?,
        };
        write_json(&mut stdout, &response)?;

        if should_shutdown {
            break;
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

fn write_json(stdout: &mut impl Write, response: &JsonResponse) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, response)
        .map_err(|error| MeridianError::InvalidMidi(error.to_string()))?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
