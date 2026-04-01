mod support;

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use meridian_core::{
    PROTOCOL_VERSION,
    protocol::{JsonRequest, JsonResponse, ProtocolCommand, ProtocolEvent},
};

fn read_response(reader: &mut BufReader<std::process::ChildStdout>) -> JsonResponse {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read response line");
    serde_json::from_str(line.trim()).expect("parse response json")
}

fn send_request(
    stdin: &mut std::process::ChildStdin,
    request: &JsonRequest,
) -> std::io::Result<()> {
    writeln!(
        stdin,
        "{}",
        serde_json::to_string(request).expect("serialize request")
    )?;
    stdin.flush()
}

#[test]
fn stdio_transport_round_trips_core_events() {
    let midi = support::write_test_midi("smoke-two-notes.mid");
    let mut child = Command::new(support::cli_path())
        .arg("stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn meridian cli");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut stdout = BufReader::new(stdout);

    send_request(
        &mut stdin,
        &JsonRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(1),
            command: ProtocolCommand::LoadParsedMidi { path: midi.clone() },
        },
    )
    .expect("send parsed midi request");

    let response = read_response(&mut stdout);
    assert_eq!(response.protocol_version, PROTOCOL_VERSION);
    assert_eq!(response.id, Some(1));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::ParsedMidiLoaded { path, .. }] if path == &midi
    ));

    send_request(
        &mut stdin,
        &JsonRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(2),
            command: ProtocolCommand::LoadAudioMidi { path: midi.clone() },
        },
    )
    .expect("send audio midi request");

    let response = read_response(&mut stdout);
    assert_eq!(response.id, Some(2));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::MidiLoaded { path }] if path == &midi
    ));

    send_request(
        &mut stdin,
        &JsonRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(3),
            command: ProtocolCommand::Shutdown,
        },
    )
    .expect("send shutdown request");

    let response = read_response(&mut stdout);
    assert_eq!(response.id, Some(3));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::ShutdownComplete]
    ));

    let status = child.wait().expect("wait for cli exit");
    assert!(status.success(), "stdio transport failed: {status:?}");
}
