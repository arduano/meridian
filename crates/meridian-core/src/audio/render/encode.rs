use std::{
    path::Path,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
};

#[cfg(unix)]
use std::process::Child;

use crate::{
    MeridianError, ffmpeg,
    midi::audio_cache::InRamAudioCache,
    protocol::{AudioOutputFormat, AudioRenderJobId},
};

use super::{
    AudioConfig,
    config::{AudioRenderConfig, ResolvedAudioRenderSettings},
    events::AudioRenderEvent,
    render_loop::{AudioRenderLoopResult, run_audio_render_loop},
    renderer::OfflineAudioRenderer,
};

use super::super::soundfont_cache::SoundfontCache;

#[expect(
    clippy::too_many_arguments,
    reason = "Offline encoded renders need these explicit collaborators and job controls at the boundary."
)]
pub(crate) fn render_encoded_audio(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    #[cfg(unix)]
    {
        render_encoded_audio_unix(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            callback,
        )
    }

    #[cfg(not(unix))]
    {
        return render_encoded_audio_fallback(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            callback,
        );
    }
}

#[cfg(unix)]
#[expect(
    clippy::too_many_arguments,
    reason = "The Unix FIFO path mirrors the encoded render boundary without additional wrapper state."
)]
fn render_encoded_audio_unix(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let fifo = ffmpeg::AudioPipeGuard::create(&render_config.output, "audio")?;
    let audio_params = settings.audio_params()?;
    let mut encoder = spawn_audio_encoder(
        &render_config.output,
        render_config.format,
        settings.sample_rate,
        settings.channels,
        &render_config.ffmpeg_args,
        fifo.path(),
    )?;
    let renderer = match OfflineAudioRenderer::new_raw_pipe(
        audio_config,
        soundfont_cache,
        fifo.path(),
        audio_params,
        settings.use_limiter,
        cancel,
    ) {
        Ok(renderer) => renderer,
        Err(error) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            return Err(error);
        }
    };
    let result = run_audio_render_loop(
        events,
        audio_config,
        renderer,
        render_config,
        job_id,
        cancel,
        callback,
    );
    match result {
        Ok(AudioRenderLoopResult::Finished {
            frames_written,
            rendered_seconds,
        }) => {
            let status = encoder.wait()?;
            if !status.success() {
                return Err(MeridianError::Platform(format!(
                    "ffmpeg exited with status {status}"
                )));
            }
            Ok(AudioRenderLoopResult::Finished {
                frames_written,
                rendered_seconds,
            })
        }
        Ok(AudioRenderLoopResult::Cancelled) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            Ok(AudioRenderLoopResult::Cancelled)
        }
        Err(error) => {
            let _ = encoder.kill();
            let _ = encoder.wait();
            Err(error)
        }
    }
}

#[cfg(not(unix))]
fn render_encoded_audio_fallback(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    settings: ResolvedAudioRenderSettings,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let temp_output = ffmpeg::next_temp_path(&render_config.output, "audio", "wav");
    let temp_guard = ffmpeg::TempPathGuard::new(temp_output.clone());
    let renderer = OfflineAudioRenderer::new_wav(
        audio_config,
        soundfont_cache,
        &temp_output,
        settings.audio_params()?,
        settings.use_limiter,
    )?;
    let result = run_audio_render_loop(
        events,
        audio_config,
        renderer,
        render_config,
        job_id,
        cancel,
        callback,
    )?;
    match result {
        AudioRenderLoopResult::Finished {
            frames_written,
            rendered_seconds,
        } => {
            encode_audio_file(
                &temp_output,
                &render_config.output,
                render_config.format,
                &render_config.ffmpeg_args,
            )?;
            drop(temp_guard);
            Ok(AudioRenderLoopResult::Finished {
                frames_written,
                rendered_seconds,
            })
        }
        AudioRenderLoopResult::Cancelled => Ok(AudioRenderLoopResult::Cancelled),
    }
}

#[cfg(unix)]
fn spawn_audio_encoder(
    output: &Path,
    format: AudioOutputFormat,
    sample_rate: u32,
    channels: u16,
    extra_args: &[String],
    input: &Path,
) -> Result<Child, MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "f32le".to_string(),
        "-ar".to_string(),
        sample_rate.to_string(),
        "-ac".to_string(),
        channels.to_string(),
        "-i".to_string(),
        input.display().to_string(),
    ];
    match format {
        AudioOutputFormat::Wav => {}
        AudioOutputFormat::Flac => {}
        AudioOutputFormat::Mp3 => {
            args.extend([
                "-codec:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
            ]);
        }
    }
    args.extend(extra_args.iter().cloned());
    args.push(output.display().to_string());

    Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| MeridianError::Platform(format!("failed to spawn ffmpeg: {error}")))
}

#[cfg(not(unix))]
fn encode_audio_file(
    input: &Path,
    output: &Path,
    format: AudioOutputFormat,
    extra_args: &[String],
) -> Result<(), MeridianError> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input.display().to_string(),
    ];
    match format {
        AudioOutputFormat::Wav => {}
        AudioOutputFormat::Flac => {}
        AudioOutputFormat::Mp3 => {
            args.extend([
                "-codec:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
            ]);
        }
    }
    args.extend(extra_args.iter().cloned());
    args.push(output.display().to_string());
    let status = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| MeridianError::Platform(format!("failed to spawn ffmpeg: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(MeridianError::Platform(format!(
            "ffmpeg exited with status {status}"
        )))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        env,
        ffi::OsString,
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        thread,
        time::Duration,
    };

    use crate::{
        audio::{AudioBackend, AudioConfig, AudioRenderEvent, SoundfontCache},
        midi::{
            MidiCacheStack,
            test_support::{TestDir, note_off, note_on, tempo, write_toolkit_midi},
        },
        protocol::{AudioOutputFormat, AudioRenderJobId},
    };

    use super::spawn_audio_encoder;
    use crate::audio::render_audio_from_cache;

    static FAKE_FFMPEG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct PathGuard {
        old_path: Option<OsString>,
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            match self.old_path.take() {
                Some(path) => unsafe { env::set_var("PATH", path) },
                None => unsafe { env::remove_var("PATH") },
            }
        }
    }

    fn with_fake_ffmpeg<T>(script: &str, run: impl FnOnce(&Path) -> T) -> T {
        let _lock = FAKE_FFMPEG_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock fake ffmpeg test mutex");

        let dir = TestDir::new("meridian-core-fake-ffmpeg");
        let script_path = dir.path("ffmpeg");
        fs::write(&script_path, script).expect("write fake ffmpeg script");
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
            .expect("mark fake ffmpeg executable");

        let old_path = env::var_os("PATH");
        let mut paths = vec![script_path.parent().expect("script parent").to_path_buf()];
        if let Some(path) = &old_path {
            paths.extend(env::split_paths(path));
        }
        let joined = env::join_paths(paths).expect("join PATH entries");
        unsafe { env::set_var("PATH", joined) };
        let guard = PathGuard { old_path };

        let result = run(script_path.parent().expect("script parent"));

        drop(guard);
        result
    }

    fn wait_for_log(path: &Path) -> String {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Ok(contents) = fs::read_to_string(path) {
                return contents;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("timed out waiting for {}", path.display());
    }

    fn long_audio_midi() -> (TestDir, PathBuf) {
        let dir = TestDir::new("meridian-core-audio-render-midi");
        let path = dir.path("input.mid");
        write_toolkit_midi(
            &path,
            96,
            &[vec![
                tempo(0, 500_000),
                note_on(0, 0, 60, 100),
                note_off(96, 0, 60),
                note_on(5_760, 0, 64, 100),
                note_off(96, 0, 64),
            ]],
        );
        (dir, path)
    }

    #[test]
    fn spawn_audio_encoder_builds_expected_mp3_command_line() {
        with_fake_ffmpeg(
            r#"#!/bin/sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf '%s\n' "$@" > "$script_dir/args.txt"
sleep 30
"#,
            |script_dir| {
                let output = script_dir.join("render.mp3");
                let input = script_dir.join("input.pipe");
                let mut child = spawn_audio_encoder(
                    &output,
                    AudioOutputFormat::Mp3,
                    44_100,
                    2,
                    &["-b:a".to_string(), "96k".to_string()],
                    &input,
                )
                .expect("spawn fake ffmpeg");

                let args = wait_for_log(&script_dir.join("args.txt"));
                assert_eq!(
                    args.lines().collect::<Vec<_>>(),
                    vec![
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-y",
                        "-f",
                        "f32le",
                        "-ar",
                        "44100",
                        "-ac",
                        "2",
                        "-i",
                        input.to_str().expect("pipe path should be utf-8"),
                        "-codec:a",
                        "libmp3lame",
                        "-q:a",
                        "2",
                        "-b:a",
                        "96k",
                        output.to_str().expect("output path should be utf-8"),
                    ]
                );

                let _ = child.kill();
                let _ = child.wait();
            },
        );
    }

    #[test]
    fn render_audio_from_cache_reports_encoder_failure_and_logs_arguments() {
        with_fake_ffmpeg(
            r#"#!/bin/sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf '%s\n' "$@" > "$script_dir/args.txt"
input=""
output=""
expect_input=0
for arg in "$@"; do
    if [ "$expect_input" -eq 1 ]; then
        input=$arg
        expect_input=0
        continue
    fi
    if [ "$arg" = "-i" ]; then
        expect_input=1
        continue
    fi
    output=$arg
done
: > "$output"
cat "$input" >/dev/null
exit 42
"#,
            |script_dir| {
                let (midi_dir, midi) = long_audio_midi();
                let midi_cache = MidiCacheStack::load(&midi).expect("load midi cache");
                let audio_config = AudioConfig {
                    backend: AudioBackend::Xsynth,
                    ..AudioConfig::default()
                };
                let soundfont_cache = SoundfontCache::new();
                let output = script_dir.join("render.mp3");
                let render_config = crate::audio::AudioRenderConfig {
                    midi_path: Some(midi),
                    audio: None,
                    output: output.clone(),
                    sample_rate: Some(44_100),
                    channels: Some(2),
                    use_limiter: Some(true),
                    format: AudioOutputFormat::Mp3,
                    ffmpeg_args: vec!["-b:a".into(), "96k".into()],
                    soundfonts: Vec::new(),
                };

                let mut events = Vec::new();
                let error = render_audio_from_cache(
                    midi_cache
                        .audio_cache()
                        .expect("build audio cache")
                        .as_ref(),
                    &audio_config,
                    &soundfont_cache,
                    &render_config,
                    AudioRenderJobId(7),
                    &std::sync::atomic::AtomicBool::new(false),
                    |event| events.push(event),
                )
                .expect_err("fake ffmpeg failure should propagate");

                drop(midi_dir);

                assert!(
                    error.to_string().contains("ffmpeg exited with status"),
                    "unexpected error: {error}"
                );
                assert!(matches!(
                    events.as_slice(),
                    [AudioRenderEvent::RenderStarted { .. }, ..]
                ));
                assert!(events.iter().any(|event| matches!(
                    event,
                    AudioRenderEvent::RenderFailed { message }
                    if message.contains("ffmpeg exited with status")
                )));

                let args = wait_for_log(&script_dir.join("args.txt"));
                assert!(args.lines().any(|line| line == "-codec:a"));
                assert!(args.lines().any(|line| line == "libmp3lame"));
                assert!(args.lines().any(|line| line == "-q:a"));
                assert!(args.lines().any(|line| line == "2"));
                assert!(args.lines().any(|line| line == "-b:a"));
                assert!(args.lines().any(|line| line == "96k"));
                assert!(
                    output.exists(),
                    "expected fake encoder to create output path"
                );
            },
        );
    }
}
