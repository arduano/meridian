use std::process::Command;

fn cli_path() -> String {
    std::env::var("CARGO_BIN_EXE_meridian-cli").expect("cargo exposes built cli path")
}

#[test]
fn top_level_help_exposes_stdio_and_render_commands() {
    let output = Command::new(cli_path())
        .arg("--help")
        .output()
        .expect("run cli --help");

    assert!(output.status.success(), "help failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf8 help");
    assert!(stdout.contains("stdio"));
    assert!(stdout.contains("analyze"));
    assert!(stdout.contains("merge"));
    assert!(stdout.contains("inspect"));
    assert!(stdout.contains("process"));
    assert!(stdout.contains("render"));
    assert!(stdout.contains("bench"));
    assert!(!stdout.contains("frame-stdout"));
}

#[test]
fn stdio_alias_serve_json_remains_available() {
    let output = Command::new(cli_path())
        .args(["serve-json", "--help"])
        .output()
        .expect("run cli serve-json --help");

    assert!(
        output.status.success(),
        "serve-json help failed: {output:?}"
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 help");
    assert!(stdout.contains("stdio"));
    assert!(!stdout.contains("serve-json"));
}
