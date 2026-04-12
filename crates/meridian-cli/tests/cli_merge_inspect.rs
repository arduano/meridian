mod support;

use std::{fs, process::Command};

#[test]
fn inspect_command_emits_inspection_json() {
    let midi = support::write_test_midi("smoke-two-notes.mid");

    let output = Command::new(support::cli_path())
        .args(["inspect", midi.to_string_lossy().as_ref(), "--pretty"])
        .output()
        .expect("run inspect command");

    assert!(output.status.success(), "inspect failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"midi_files_inspected\""));
    assert!(stdout.contains("\"declared_track_count\""));
    assert!(stdout.contains("\"total_notes\""));
}

#[test]
fn merge_command_writes_output_file() {
    let left = support::write_test_midi("smoke-two-notes.mid");
    let right = support::write_test_midi("smoke-two-notes-copy.mid");
    let out = support::temp_path("merged.mid");

    let output = Command::new(support::cli_path())
        .args([
            "merge",
            left.to_string_lossy().as_ref(),
            right.to_string_lossy().as_ref(),
            "--output",
            out.to_string_lossy().as_ref(),
            "--pretty",
        ])
        .output()
        .expect("run merge command");

    assert!(output.status.success(), "merge failed: {output:?}");
    assert!(out.exists(), "expected output midi to exist");
    assert!(fs::metadata(&out).expect("merged metadata").len() > 0);
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"midi_files_merged\""));
    assert!(stdout.contains("\"output_track_count\""));
}
