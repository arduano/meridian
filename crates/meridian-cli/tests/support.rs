#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn cli_path() -> String {
    std::env::var("CARGO_BIN_EXE_meridian-cli").expect("cargo exposes built cli path")
}

pub fn temp_path(name: &str) -> PathBuf {
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

fn repo_midis_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/midis")
}

fn repo_soundfonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/soundfonts")
}

pub fn fixture_or_temp(name: &str, bytes: &[u8]) -> PathBuf {
    for candidate in [
        repo_midis_dir().join(name),
        repo_midis_dir().join("test").join(name),
        repo_midis_dir().join("piano").join(name),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }

    let path = temp_path(name);
    fs::write(&path, bytes).expect("write midi fixture");
    path
}

pub fn write_fixture_midi(path: &Path) {
    let bytes = [
        0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x4d,
        0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00,
        0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30, 0x80, 0x40, 0x40,
        0x00, 0xff, 0x2f, 0x00,
    ];
    fs::write(path, bytes).expect("write midi fixture");
}

pub fn write_test_midi(name: &str) -> PathBuf {
    fixture_or_temp(
        name,
        &[
            0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1,
            0x20, 0x00, 0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30,
            0x80, 0x40, 0x40, 0x00, 0xff, 0x2f, 0x00,
        ],
    )
}

pub fn soundfont_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MERIDIAN_TEST_SOUNDFONT")
        .or_else(|| std::env::var_os("MERIDIAN_SOUNDFONT"))
    {
        return Some(PathBuf::from(path));
    }

    let bundled = repo_soundfonts_dir()
        .join("freepats-upright-kw-small")
        .join("UprightPianoKW-small-20190703.sfz");
    bundled.exists().then_some(bundled)
}

pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
