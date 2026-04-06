mod support;

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use meridian_core::{
    PROTOCOL_VERSION,
    protocol::{
        FrameColorMode, ProtocolCommand, ProtocolEvent, ProtocolRequest, ProtocolResponse,
        VideoRenderEvent,
    },
    render::{
        FlatKeyboardProjectorConfig, FlatNoteProjectorConfig, KeyboardProjectorConfig,
        NoteProjectorConfig, SceneConfig, TwoDSceneConfig,
    },
};

fn read_response(reader: &mut BufReader<std::process::ChildStdout>) -> ProtocolResponse {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read response line");
    serde_json::from_str(line.trim()).expect("parse response json")
}

fn read_response_for_id(
    reader: &mut BufReader<std::process::ChildStdout>,
    id: u64,
) -> ProtocolResponse {
    loop {
        let response = read_response(reader);
        if response.id == Some(id) {
            return response;
        }
    }
}

fn send_request(
    stdin: &mut std::process::ChildStdin,
    request: &ProtocolRequest,
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
        &ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(1),
            command: ProtocolCommand::LoadParsedMidi { path: midi.clone() },
        },
    )
    .expect("send parsed midi request");

    let response = read_response_for_id(&mut stdout, 1);
    assert_eq!(response.protocol_version, PROTOCOL_VERSION);
    assert_eq!(response.id, Some(1));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::ParsedMidiLoaded { path, .. }] if path == &midi
    ));

    send_request(
        &mut stdin,
        &ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(2),
            command: ProtocolCommand::LoadAudioMidi { path: midi.clone() },
        },
    )
    .expect("send audio midi request");

    let response = read_response_for_id(&mut stdout, 2);
    assert_eq!(response.id, Some(2));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::MidiLoaded { path }] if path == &midi
    ));

    send_request(
        &mut stdin,
        &ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(3),
            command: ProtocolCommand::Shutdown,
        },
    )
    .expect("send shutdown request");

    let response = read_response_for_id(&mut stdout, 3);
    assert_eq!(response.id, Some(3));
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::ShutdownComplete]
    ));

    let status = child.wait().expect("wait for cli exit");
    assert!(status.success(), "stdio transport failed: {status:?}");
}

#[test]
fn stdio_transport_saves_frame_with_alpha_sidecars() {
    let midi = support::write_test_midi("smoke-two-notes.mid");
    let output = support::temp_path("frame.png");
    let premultiplied = output.with_file_name("frame.premultiplied.png");
    let straight = output.with_file_name("frame.straight.png");
    let alpha = output.with_file_name("frame.alpha.png");

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

    for (id, command) in [
        (1, ProtocolCommand::LoadMidi { path: midi.clone() }),
        (2, ProtocolCommand::SetTime { time: 0.25 }),
        (
            3,
            ProtocolCommand::SetViewport {
                width: 320,
                height: 180,
            },
        ),
        (
            4,
            ProtocolCommand::SetSceneConfig {
                scene: SceneConfig::TwoD(TwoDSceneConfig {
                    notes: NoteProjectorConfig::Flat(FlatNoteProjectorConfig::default()),
                    keyboard: KeyboardProjectorConfig::Flat(FlatKeyboardProjectorConfig),
                    ..Default::default()
                }),
            },
        ),
        (
            5,
            ProtocolCommand::SaveFrame {
                output: output.clone(),
                format: None,
                viewport_width: Some(320),
                viewport_height: Some(180),
                export: meridian_core::protocol::ImageExportConfig {
                    color_mode: FrameColorMode::Premultiplied,
                    export_premultiplied_rgb: true,
                    export_straight_rgb: true,
                    export_alpha_mask: true,
                },
            },
        ),
        (6, ProtocolCommand::Shutdown),
    ] {
        send_request(
            &mut stdin,
            &ProtocolRequest {
                protocol_version: PROTOCOL_VERSION,
                id: Some(id),
                command,
            },
        )
        .expect("send stdio request");
        let response = read_response_for_id(&mut stdout, id);
        assert_eq!(response.id, Some(id));
        if id == 5 {
            assert!(matches!(
                response.events.as_slice(),
                [ProtocolEvent::FrameSaved { output: saved_output, exports, .. }]
                    if saved_output == &output
                    && exports.premultiplied_rgb.as_ref() == Some(&premultiplied)
                    && exports.straight_rgb.as_ref() == Some(&straight)
                    && exports.alpha_mask.as_ref() == Some(&alpha)
            ));
        }
    }

    for path in [&output, &premultiplied, &straight, &alpha] {
        assert!(path.exists(), "expected {} to exist", path.display());
    }

    let status = child.wait().expect("wait for cli exit");
    assert!(status.success(), "stdio transport failed: {status:?}");
}

#[test]
fn stdio_transport_renders_video_with_alpha_mask() {
    if !support::ffmpeg_available() {
        eprintln!("skipping stdio alpha video smoke because ffmpeg is unavailable");
        return;
    }

    let midi = support::write_test_midi("piano/burgmuller-op100-no4-the-little-party.mid");
    let output = support::temp_path("render.mp4");
    let alpha = output.with_file_name("render.alpha.mp4");

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
        &ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(1),
            command: ProtocolCommand::StartRenderVideo {
                config: meridian_core::protocol::ProtocolVideoRenderConfig {
                    midi_path: midi,
                    output: output.clone(),
                    fps: 4.0,
                    width: 160,
                    height: 90,
                    renderer: Some(meridian_core::render::RendererKind::PianoTrailClassic),
                    scene: None,
                    view_range: Some(2.0),
                    time_space: None,
                    first_key: None,
                    last_key: None,
                    ffmpeg_args: vec!["-y".to_string()],
                    export: meridian_core::protocol::VideoExportConfig {
                        color_mode: FrameColorMode::Premultiplied,
                        export_alpha_mask: true,
                    },
                },
            },
        },
    )
    .expect("send start_render_video request");

    let response = read_response_for_id(&mut stdout, 1);
    assert!(matches!(
        response.events.as_slice(),
        [ProtocolEvent::VideoRenderStatus { status }]
            if matches!(status, meridian_core::protocol::VideoRenderStatus::Running { .. })
    ));

    let mut saw_finished = false;
    loop {
        let response = read_response(&mut stdout);
        for event in response.events {
            match event {
                ProtocolEvent::VideoRender {
                    event:
                        VideoRenderEvent::RenderFinished {
                            output: finished_output,
                            exports,
                            ..
                        },
                } => {
                    assert_eq!(finished_output, output);
                    assert_eq!(exports.alpha_mask.as_ref(), Some(&alpha));
                    saw_finished = true;
                    break;
                }
                ProtocolEvent::VideoRender {
                    event: VideoRenderEvent::RenderFailed { message },
                } => panic!("video render failed: {message}"),
                _ => {}
            }
        }
        if saw_finished {
            break;
        }
    }

    send_request(
        &mut stdin,
        &ProtocolRequest {
            protocol_version: PROTOCOL_VERSION,
            id: Some(2),
            command: ProtocolCommand::Shutdown,
        },
    )
    .expect("send shutdown");
    let _ = read_response_for_id(&mut stdout, 2);

    assert!(output.exists(), "expected video output to exist");
    assert!(alpha.exists(), "expected alpha video output to exist");
    assert!(std::fs::metadata(&output).expect("video metadata").len() > 0);
    assert!(
        std::fs::metadata(&alpha)
            .expect("alpha video metadata")
            .len()
            > 0
    );

    let status = child.wait().expect("wait for cli exit");
    assert!(status.success(), "stdio transport failed: {status:?}");
}
