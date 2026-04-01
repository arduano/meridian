use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn cli_path() -> String {
    std::env::var("CARGO_BIN_EXE_meridian-cli").expect("cargo exposes built cli path")
}

fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "meridian-cli-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir.join(name)
}

fn write_fixture_midi(path: &PathBuf) {
    let bytes = [
        0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x4d,
        0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00,
        0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30, 0x80, 0x40, 0x40,
        0x00, 0xff, 0x2f, 0x00,
    ];
    fs::write(path, bytes).expect("write midi fixture");
}

#[test]
fn analyze_command_emits_analysis_json() {
    let midi = temp_path("fixture.mid");
    write_fixture_midi(&midi);

    let output = Command::new(cli_path())
        .args([
            "analyze",
            midi.to_string_lossy().as_ref(),
            "--pretty",
            "--buckets",
            "4",
        ])
        .output()
        .expect("run analyze command");

    assert!(output.status.success(), "analyze failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"total_notes\": 2"));
    assert!(stdout.contains("\"buckets\":"));
}

#[test]
fn process_tempo_flatten_writes_output_file() {
    let midi = temp_path("input.mid");
    let out = temp_path("out.mid");
    write_fixture_midi(&midi);

    let output = Command::new(cli_path())
        .args([
            "process",
            "tempo-flatten",
            midi.to_string_lossy().as_ref(),
            "--output",
            out.to_string_lossy().as_ref(),
            "--tempo",
            "500000",
            "--pretty",
        ])
        .output()
        .expect("run process command");

    assert!(output.status.success(), "process failed: {output:?}");
    assert!(out.exists(), "expected output midi to exist");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"process_finished\""));
    assert!(stdout.contains("\"output_track_count\""));
}
