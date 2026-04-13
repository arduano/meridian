mod support;

use std::process::Command;

#[test]
fn analyze_command_emits_analysis_json() {
    let midi = support::write_test_midi("smoke-two-notes.mid");

    let output = Command::new(support::cli_path())
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
    let midi = support::write_test_midi("smoke-two-notes.mid");
    let out = support::temp_path("out.mid");

    let output = Command::new(support::cli_path())
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
