use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use meridian_core::{
    PROTOCOL_VERSION,
    protocol::{CoreCommand, CoreEvent, JsonRequest, JsonResponse},
    spawn_core,
};

fn write_test_midi() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "meridian-core-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
    let path = dir.join("fixture.mid");
    let bytes = [
        0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x4d,
        0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00,
        0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30, 0x80, 0x40, 0x40,
        0x00, 0xff, 0x2f, 0x00,
    ];
    fs::write(&path, bytes).expect("write midi fixture");
    path
}

#[test]
fn stateful_core_projects_a_frame() {
    let midi = write_test_midi();
    let core = spawn_core();

    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetTime { time: 0.25 })
        .expect("set time");

    let frame = core
        .render_frame(Some(320), Some(180))
        .expect("render frame");

    assert_eq!(frame.state.total_notes, 2);
    assert_eq!(frame.stats.visible_notes, 2);
    assert_eq!(frame.stats.active_keys, 2);
    assert!(frame.stats.total_quads > 0);
    assert!(frame.scene.total_quads() > 0);
}

#[test]
fn save_frame_supports_png_and_rgba() {
    let midi = write_test_midi();
    let out_dir = midi.parent().unwrap().to_path_buf();
    let png_path = out_dir.join("frame.png");
    let rgba_path = out_dir.join("frame.rgba");
    let core = spawn_core();

    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetTime { time: 0.25 })
        .expect("set time");

    let png_events = core
        .request(CoreCommand::SaveFrame {
            output: png_path.clone(),
            format: None,
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("save png");
    let rgba_events = core
        .request(CoreCommand::SaveFrame {
            output: rgba_path.clone(),
            format: None,
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("save rgba");

    let png_bytes = fs::read(&png_path).expect("read png");
    let rgba_bytes = fs::read(&rgba_path).expect("read rgba");

    assert!(matches!(
        png_events.as_slice(),
        [CoreEvent::FrameSaved { bytes_written, .. }] if *bytes_written > 0
    ));
    assert!(matches!(
        rgba_events.as_slice(),
        [CoreEvent::FrameSaved { bytes_written, .. }] if *bytes_written > 0
    ));
    assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(rgba_bytes.len(), 320 * 180 * 4);
}

#[test]
fn json_protocol_defaults_version_and_render_response_is_lightweight() {
    let midi = write_test_midi();
    let core = spawn_core();
    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");

    let request: JsonRequest =
        serde_json::from_str(r#"{"id":7,"command":{"type":"get_state"}}"#).expect("json request");
    assert_eq!(request.protocol_version, PROTOCOL_VERSION);

    let events = core
        .request(CoreCommand::RenderFrame {
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("render stats");
    let response = JsonResponse {
        protocol_version: PROTOCOL_VERSION,
        id: Some(7),
        events,
    };
    let json = serde_json::to_string(&response).expect("serialize response");

    assert!(json.contains("\"frame_projected\""));
    assert!(!json.contains("\"scene\""));
}
