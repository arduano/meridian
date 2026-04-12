mod support;

use std::{fs, process::Command};

use meridian_core::protocol::AudioOutputFormat;

#[test]
fn render_video_writes_output_file_when_ffmpeg_is_available() {
    if !support::ffmpeg_available() {
        eprintln!("skipping CLI video render smoke because ffmpeg is unavailable");
        return;
    }

    let midi = support::write_test_midi("piano/burgmuller-op100-no4-the-little-party.mid");
    let out = support::temp_path("render.mkv");

    let output = Command::new(support::cli_path())
        .args([
            "render",
            "video",
            midi.to_string_lossy().as_ref(),
            "--output",
            out.to_string_lossy().as_ref(),
            "--fps",
            "4",
            "--container",
            "mkv",
            "--width",
            "160",
            "--height",
            "90",
            "--view-range",
            "2",
            "--renderer",
            "piano-trail-classic",
            "--ffmpeg-flags",
            "-y",
        ])
        .output()
        .expect("run render video command");

    assert!(output.status.success(), "render video failed: {output:?}");
    assert!(out.exists(), "expected output video to exist");
    assert!(fs::metadata(&out).expect("video metadata").len() > 0);
}

#[test]
fn render_audio_writes_output_file_when_soundfont_is_available() {
    render_audio_smoke(
        AudioOutputFormat::Wav,
        "render.wav",
        &["--format", "wav"],
        |bytes| assert!(bytes.starts_with(b"RIFF"), "expected wav output"),
    );
}

#[test]
fn render_audio_can_write_flac_when_requested() {
    render_audio_smoke(
        AudioOutputFormat::Flac,
        "render.flac",
        &["--format", "flac"],
        |bytes| assert!(bytes.starts_with(b"fLaC"), "expected flac output"),
    );
}

#[test]
fn render_audio_can_write_mp3_when_requested() {
    render_audio_smoke(
        AudioOutputFormat::Mp3,
        "render.mp3",
        &["--format", "mp3"],
        |bytes| {
            assert!(
                bytes.starts_with(b"ID3") || bytes.first() == Some(&0xFF),
                "expected mp3 output"
            );
        },
    );
}

fn render_audio_smoke(
    format: AudioOutputFormat,
    output_name: &str,
    format_args: &[&str],
    assert_magic: impl FnOnce(&[u8]),
) {
    let Some(soundfont) = support::soundfont_path() else {
        eprintln!("skipping CLI audio render smoke because MERIDIAN_TEST_SOUNDFONT is unset");
        return;
    };

    let midi = support::write_test_midi("piano/burgmuller-op100-no13-consolation.mid");
    let out = support::temp_path(output_name);
    let midi = midi.to_string_lossy().to_string();
    let out = out.to_string_lossy().to_string();
    let soundfont = soundfont.to_string_lossy().to_string();

    let mut args = vec![
        "render",
        "audio",
        midi.as_str(),
        "--output",
        out.as_str(),
        "--sample-rate",
        "22050",
        "--channels",
        "2",
        "--soundfont",
        soundfont.as_str(),
    ];
    args.extend(format_args.iter().copied());

    let output = Command::new(support::cli_path())
        .args(args)
        .output()
        .expect("run render audio command");

    assert!(
        output.status.success(),
        "render audio {:?} failed: {output:?}",
        format
    );
    let bytes = fs::read(&out).expect("read audio output");
    assert!(bytes.len() > 12, "expected non-empty audio output");
    assert_magic(&bytes);
}

#[test]
fn render_audio_uses_embedded_default_soundfont_when_no_soundfont_is_passed() {
    let midi = support::write_test_midi("burgmuller-op100-no13-consolation-default.mid");
    let out = support::temp_path("render-default.wav");

    let output = Command::new(support::cli_path())
        .args([
            "render",
            "audio",
            midi.to_string_lossy().as_ref(),
            "--output",
            out.to_string_lossy().as_ref(),
            "--format",
            "wav",
            "--sample-rate",
            "22050",
            "--channels",
            "2",
        ])
        .output()
        .expect("run render audio command with embedded default soundfont");

    assert!(
        output.status.success(),
        "render audio with embedded default failed: {output:?}"
    );
    let bytes = fs::read(&out).expect("read embedded-default audio output");
    assert!(bytes.len() > 12, "expected non-empty wav output");
    assert_eq!(&bytes[..4], b"RIFF");
}
