use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    ops::RangeInclusive,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};

use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use xsynth_core::{
    AudioPipe, AudioStreamParams, ChannelCount,
    channel::{ChannelAudioEvent, ChannelConfigEvent, ChannelEvent, ControlEvent},
    channel_group::{ChannelGroup, ChannelGroupConfig, SynthEvent},
    effects::VolumeLimiter,
};

use crate::{
    MeridianError,
    midi::{MidiCacheStack, audio_cache::InRamAudioCache},
    protocol::{AudioOutputFormat, AudioRenderJobId, VideoAudioConfig, VideoAudioProgress},
};

use super::{AudioBackend, AudioConfig, soundfont_cache::SoundfontCache};
use crate::audio::MeridianSoundfont;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioRenderConfig {
    pub midi_path: Option<PathBuf>,
    pub audio: Option<AudioConfig>,
    pub output: PathBuf,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub use_limiter: Option<bool>,
    #[serde(default)]
    pub format: AudioOutputFormat,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
    #[serde(default)]
    pub soundfonts: Vec<PathBuf>,
}

impl Default for AudioRenderConfig {
    fn default() -> Self {
        Self {
            midi_path: None,
            audio: None,
            output: PathBuf::from("out.wav"),
            sample_rate: None,
            channels: None,
            use_limiter: None,
            format: AudioOutputFormat::Wav,
            ffmpeg_args: Vec::new(),
            soundfonts: Vec::new(),
        }
    }
}

pub(crate) fn resolve_video_audio_settings(
    audio_config: &AudioConfig,
    config: &VideoAudioConfig,
) -> Result<(u32, u16, bool), MeridianError> {
    let sample_rate = config
        .sample_rate
        .unwrap_or(audio_config.xsynth.render.audio_params.sample_rate);
    let channels = config
        .channels
        .unwrap_or(audio_config.xsynth.render.audio_params.channels.count());
    let use_limiter = config
        .use_limiter
        .unwrap_or(audio_config.xsynth.render.use_limiter);
    ChannelCount::from_count(channels).ok_or_else(|| {
        MeridianError::Platform(format!(
            "unsupported channel count {}; only 1 or 2 are supported",
            channels
        ))
    })?;
    Ok((sample_rate, channels, use_limiter))
}

pub(crate) fn render_audio_pipe_from_cache(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &VideoAudioConfig,
    pipe_path: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(VideoAudioProgress),
) -> Result<(), MeridianError> {
    let (sample_rate, channels, use_limiter) = resolve_video_audio_settings(audio_config, config)?;
    let audio_params = AudioStreamParams::new(
        sample_rate,
        ChannelCount::from_count(channels).expect("validated channel count"),
    );
    let mut render_audio_config = audio_config.clone();
    if !config.soundfonts.is_empty() {
        render_audio_config.soundfonts = config
            .soundfonts
            .iter()
            .cloned()
            .map(|path| MeridianSoundfont {
                path: Some(path),
                ..MeridianSoundfont::default()
            })
            .collect();
    }
    let renderer = OfflineAudioRenderer::new_raw_pipe(
        &render_audio_config,
        soundfont_cache,
        pipe_path,
        audio_params,
        use_limiter,
    )?;
    let noop_config = AudioRenderConfig {
        output: pipe_path.to_path_buf(),
        ..AudioRenderConfig::default()
    };
    let total_events = events.events().len();
    on_progress(VideoAudioProgress {
        total_events,
        event_index: 0,
        rendered_seconds: 0.0,
    });
    let result = run_audio_render_loop(
        events,
        &render_audio_config,
        renderer,
        &noop_config,
        AudioRenderJobId(0),
        cancel,
        &mut |event| {
            if let AudioRenderEvent::RenderProgress {
                total_events,
                event_index,
                rendered_seconds,
                ..
            } = event
            {
                on_progress(VideoAudioProgress {
                    total_events,
                    event_index,
                    rendered_seconds,
                });
            }
        },
    )?;
    match result {
        AudioRenderLoopResult::Finished {
            rendered_seconds, ..
        } => {
            on_progress(VideoAudioProgress {
                total_events,
                event_index: total_events,
                rendered_seconds,
            });
            Ok(())
        }
        AudioRenderLoopResult::Cancelled => Ok(()),
    }
}

pub fn render_audio(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    on_event: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    let events = midi_cache.audio_cache()?;
    render_audio_from_cache(
        events.as_ref(),
        audio_config,
        soundfont_cache,
        config,
        job_id,
        cancel,
        on_event,
    )
}

pub fn render_audio_from_cache(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_audio_inner(
            events,
            audio_config,
            soundfont_cache,
            config,
            job_id,
            cancel,
            &mut on_event,
        )
    }))
    .map_err(|_| MeridianError::Platform("xsynth panicked during offline audio render".into()))?
    .map_err(|error| {
        on_event(AudioRenderEvent::RenderFailed {
            message: error.to_string(),
        });
        error
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioRenderEvent {
    RenderStarted {
        job_id: AudioRenderJobId,
        output: PathBuf,
        sample_rate: u32,
        channels: u16,
        total_events: usize,
    },
    RenderProgress {
        job_id: AudioRenderJobId,
        event_index: usize,
        total_events: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
        voice_count: u64,
    },
    RenderFinished {
        job_id: AudioRenderJobId,
        output: PathBuf,
        frames_written: u64,
        rendered_seconds: f64,
    },
    RenderCancelled {
        job_id: AudioRenderJobId,
        output: PathBuf,
        event_index: usize,
        total_events: usize,
        rendered_seconds: f64,
        frames_written: u64,
    },
    RenderFailed {
        message: String,
    },
}

pub fn render_audio_to_wav(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    callback: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    let cancel = AtomicBool::new(false);
    render_audio(
        midi_cache,
        audio_config,
        soundfont_cache,
        render_config,
        AudioRenderJobId(0),
        &cancel,
        callback,
    )
}

fn render_audio_inner(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    mut callback: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    if matches!(audio_config.backend, AudioBackend::None) {
        return Err(MeridianError::Platform(
            "audio backend 'none' cannot render audio".into(),
        ));
    }

    let settings = resolve_render_settings(audio_config, render_config)?;

    callback(AudioRenderEvent::RenderStarted {
        job_id,
        output: render_config.output.clone(),
        sample_rate: settings.sample_rate,
        channels: settings.channels,
        total_events: events.events().len(),
    });

    let outcome = match render_config.format {
        AudioOutputFormat::Wav => {
            let renderer = OfflineAudioRenderer::new_wav(
                audio_config,
                soundfont_cache,
                &render_config.output,
                settings.audio_params()?,
                settings.use_limiter,
            )?;
            run_audio_render_loop(
                events,
                audio_config,
                renderer,
                render_config,
                job_id,
                cancel,
                &mut callback,
            )?
        }
        AudioOutputFormat::Flac | AudioOutputFormat::Mp3 => render_encoded_audio(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            &mut callback,
        )?,
    };

    if let AudioRenderLoopResult::Finished {
        frames_written,
        rendered_seconds,
    } = outcome
    {
        callback(AudioRenderEvent::RenderFinished {
            job_id,
            output: render_config.output.clone(),
            frames_written,
            rendered_seconds,
        });
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ResolvedAudioRenderSettings {
    sample_rate: u32,
    channels: u16,
    use_limiter: bool,
}

impl ResolvedAudioRenderSettings {
    fn audio_params(self) -> Result<AudioStreamParams, MeridianError> {
        let channel_count = ChannelCount::from_count(self.channels).ok_or_else(|| {
            MeridianError::Platform(format!(
                "unsupported channel count {}; only 1 or 2 are supported",
                self.channels
            ))
        })?;
        Ok(AudioStreamParams::new(self.sample_rate, channel_count))
    }
}

fn resolve_render_settings(
    audio_config: &AudioConfig,
    render_config: &AudioRenderConfig,
) -> Result<ResolvedAudioRenderSettings, MeridianError> {
    let sample_rate = render_config
        .sample_rate
        .unwrap_or(audio_config.xsynth.render.audio_params.sample_rate);
    let channels = render_config
        .channels
        .unwrap_or(audio_config.xsynth.render.audio_params.channels.count());
    let use_limiter = render_config
        .use_limiter
        .unwrap_or(audio_config.xsynth.render.use_limiter);

    ChannelCount::from_count(channels).ok_or_else(|| {
        MeridianError::Platform(format!(
            "unsupported channel count {}; only 1 or 2 are supported",
            channels
        ))
    })?;

    Ok(ResolvedAudioRenderSettings {
        sample_rate,
        channels,
        use_limiter,
    })
}

enum AudioRenderLoopResult {
    Finished {
        frames_written: u64,
        rendered_seconds: f64,
    },
    Cancelled,
}

fn run_audio_render_loop(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    mut renderer: OfflineAudioRenderer,
    render_config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let mut current_time = 0.0;
    let total_events = events.events().len();
    let total_duration_seconds = events
        .events()
        .last()
        .map(|event| event.time)
        .unwrap_or(0.0);
    let progress_stride_seconds = progress_stride_seconds(total_duration_seconds);
    let mut next_progress_seconds = progress_stride_seconds;
    for (event_index, event) in events.events().iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            callback(AudioRenderEvent::RenderCancelled {
                job_id,
                output: render_config.output.clone(),
                event_index,
                total_events,
                rendered_seconds: current_time,
                frames_written: renderer.frames_written(),
            });
            return Ok(AudioRenderLoopResult::Cancelled);
        }

        while current_time + f64::EPSILON < event.time {
            if cancel.load(Ordering::SeqCst) {
                callback(AudioRenderEvent::RenderCancelled {
                    job_id,
                    output: render_config.output.clone(),
                    event_index,
                    total_events,
                    rendered_seconds: current_time,
                    frames_written: renderer.frames_written(),
                });
                return Ok(AudioRenderLoopResult::Cancelled);
            }

            let next_time = next_progress_seconds.min(event.time);
            let delta = (next_time - current_time).max(0.0);
            if delta <= 0.0 {
                break;
            }

            renderer.render_batch(delta)?;
            current_time = next_time;

            if current_time + f64::EPSILON >= next_progress_seconds {
                emit_audio_render_progress(
                    callback,
                    job_id,
                    event_index,
                    total_events,
                    current_time,
                    current_time,
                    &renderer,
                );
                next_progress_seconds += progress_stride_seconds;
            }
        }

        for packed in event.iter_events() {
            dispatch_packed_event(
                &mut renderer,
                packed,
                &audio_config.xsynth.config.ignore_range,
            );
        }

        if event_index == 0 || event_index + 1 == total_events {
            emit_audio_render_progress(
                callback,
                job_id,
                event_index + 1,
                total_events,
                event.time,
                current_time,
                &renderer,
            );
        }
    }

    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::AllNotesOff,
    )));
    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::ResetControl,
    )));
    let frames_written = renderer.finalize()?;
    Ok(AudioRenderLoopResult::Finished {
        frames_written,
        rendered_seconds: current_time,
    })
}

fn emit_audio_render_progress(
    callback: &mut impl FnMut(AudioRenderEvent),
    job_id: AudioRenderJobId,
    event_index: usize,
    total_events: usize,
    time_seconds: f64,
    rendered_seconds: f64,
    renderer: &OfflineAudioRenderer,
) {
    callback(AudioRenderEvent::RenderProgress {
        job_id,
        event_index,
        total_events,
        time_seconds,
        rendered_seconds,
        frames_written: renderer.frames_written(),
        voice_count: renderer.voice_count(),
    });
}

struct OfflineAudioRenderer {
    channel_group: ChannelGroup,
    writer: AudioSampleWriter,
    limiter: Option<VolumeLimiter>,
    output_vec: Vec<f32>,
    missed_samples: f64,
    sample_rate: u32,
    channels: u16,
    frames_written: u64,
}

impl OfflineAudioRenderer {
    fn new_wav(
        audio_config: &AudioConfig,
        soundfont_cache: &SoundfontCache,
        output: &Path,
        audio_params: AudioStreamParams,
        use_limiter: bool,
    ) -> Result<Self, MeridianError> {
        let writer = AudioSampleWriter::create_wav(output, audio_params)?;
        Self::new_with_writer(
            audio_config,
            soundfont_cache,
            writer,
            audio_params,
            use_limiter,
        )
    }

    fn new_raw_pipe(
        audio_config: &AudioConfig,
        soundfont_cache: &SoundfontCache,
        pipe_path: &Path,
        audio_params: AudioStreamParams,
        use_limiter: bool,
    ) -> Result<Self, MeridianError> {
        let writer = AudioSampleWriter::create_raw_pipe(pipe_path)?;
        Self::new_with_writer(
            audio_config,
            soundfont_cache,
            writer,
            audio_params,
            use_limiter,
        )
    }

    fn new_with_writer(
        audio_config: &AudioConfig,
        soundfont_cache: &SoundfontCache,
        writer: AudioSampleWriter,
        audio_params: AudioStreamParams,
        use_limiter: bool,
    ) -> Result<Self, MeridianError> {
        let group_options = ChannelGroupConfig {
            channel_init_options: audio_config.xsynth.config.channel_init_options.clone(),
            format: audio_config.xsynth.config.format,
            audio_params,
            parallelism: audio_config.xsynth.render.parallelism,
        };
        let soundfonts = soundfont_cache.load_enabled(&audio_config.soundfonts, audio_params)?;
        let layers = if audio_config.xsynth.limit_layers {
            Some(audio_config.xsynth.layers)
        } else {
            None
        };
        let mut channel_group = ChannelGroup::new(group_options);
        channel_group.send_event(SynthEvent::AllChannels(ChannelEvent::Config(
            ChannelConfigEvent::SetLayerCount(layers),
        )));
        channel_group.send_event(SynthEvent::AllChannels(ChannelEvent::Config(
            ChannelConfigEvent::SetSoundfonts(soundfonts),
        )));
        Ok(Self {
            channel_group,
            writer,
            limiter: use_limiter.then(|| VolumeLimiter::new(audio_params.channels.count())),
            output_vec: vec![0.0],
            missed_samples: 0.0,
            sample_rate: audio_params.sample_rate,
            channels: audio_params.channels.count(),
            frames_written: 0,
        })
    }

    fn render_batch(&mut self, seconds: f64) -> Result<(), MeridianError> {
        if seconds > 10.0 {
            let mut remaining = seconds;
            while remaining > 0.0 {
                let next = remaining.min(10.0);
                self.render_batch(next)?;
                remaining -= next;
            }
            return Ok(());
        }

        let samples = self.sample_rate as f64 * seconds + self.missed_samples;
        self.missed_samples = samples % 1.0;
        let sample_count = samples as usize * self.channels as usize;
        self.output_vec.resize(sample_count, 0.0);
        self.channel_group.read_samples(&mut self.output_vec);
        if let Some(limiter) = &mut self.limiter {
            limiter.limit(&mut self.output_vec);
        }
        self.writer.write_samples(&self.output_vec)?;
        self.frames_written += sample_count as u64 / self.channels as u64;
        Ok(())
    }

    fn finalize(mut self) -> Result<u64, MeridianError> {
        loop {
            self.output_vec
                .resize(self.sample_rate as usize * self.channels as usize, 0.0);
            self.channel_group.read_samples(&mut self.output_vec);
            if let Some(limiter) = &mut self.limiter {
                limiter.limit(&mut self.output_vec);
            }
            let is_silent = self
                .output_vec
                .iter()
                .all(|sample| (-0.0001..=0.0001).contains(sample));
            if is_silent {
                break;
            }
            self.writer.write_samples(&self.output_vec)?;
            self.frames_written += self.output_vec.len() as u64 / self.channels as u64;
        }
        self.writer.finalize()?;
        Ok(self.frames_written)
    }

    fn send_event(&mut self, event: SynthEvent) {
        self.channel_group.send_event(event);
    }

    fn voice_count(&self) -> u64 {
        self.channel_group.voice_count()
    }

    fn frames_written(&self) -> u64 {
        self.frames_written
    }
}

enum AudioSampleWriter {
    Wav(WavWriter<BufWriter<File>>),
    RawF32(BufWriter<File>),
}

impl AudioSampleWriter {
    fn create_wav(output: &Path, audio_params: AudioStreamParams) -> Result<Self, MeridianError> {
        let spec = WavSpec {
            channels: audio_params.channels.count(),
            sample_rate: audio_params.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let writer = WavWriter::create(output, spec).map_err(|error| {
            MeridianError::Platform(format!(
                "failed to create wav writer {}: {error}",
                output.display()
            ))
        })?;
        Ok(Self::Wav(writer))
    }

    fn create_raw_pipe(pipe_path: &Path) -> Result<Self, MeridianError> {
        let writer = OpenOptions::new()
            .write(true)
            .open(pipe_path)
            .map_err(|error| {
                MeridianError::Platform(format!(
                    "failed to open audio pipe {}: {error}",
                    pipe_path.display()
                ))
            })?;
        Ok(Self::RawF32(BufWriter::new(writer)))
    }

    fn write_samples(&mut self, samples: &[f32]) -> Result<(), MeridianError> {
        match self {
            Self::Wav(writer) => {
                for sample in samples {
                    writer.write_sample(*sample).map_err(|error| {
                        MeridianError::Platform(format!("failed to write wav sample: {error}"))
                    })?;
                }
            }
            Self::RawF32(writer) => {
                for sample in samples {
                    writer.write_all(&sample.to_le_bytes()).map_err(|error| {
                        MeridianError::Platform(format!(
                            "failed to write audio pipe sample: {error}"
                        ))
                    })?;
                }
            }
        }
        Ok(())
    }

    fn finalize(self) -> Result<(), MeridianError> {
        match self {
            Self::Wav(writer) => writer.finalize().map_err(|error| {
                MeridianError::Platform(format!("failed to finalize xsynth wav render: {error}"))
            }),
            Self::RawF32(mut writer) => writer.flush().map_err(Into::into),
        }
    }
}

fn render_encoded_audio(
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
        return render_encoded_audio_unix(
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
    let fifo = crate::ffmpeg::FifoGuard::create(&render_config.output, "audio")?;
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
    let temp_output = crate::ffmpeg::next_temp_path(&render_config.output, "audio", "wav");
    let temp_guard = crate::ffmpeg::TempPathGuard::new(temp_output.clone());
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

fn progress_stride_seconds(total_duration_seconds: f64) -> f64 {
    (total_duration_seconds / 400.0).clamp(0.05, 1.0)
}

fn dispatch_packed_event(
    renderer: &mut OfflineAudioRenderer,
    packed: u32,
    ignore_range: &RangeInclusive<u8>,
) {
    let status = (packed & 0xFF) as u8;
    let data1 = ((packed >> 8) & 0xFF) as u8;
    let data2 = ((packed >> 16) & 0xFF) as u8;
    let event = status & 0xF0;
    let channel = (status & 0x0F) as u32;
    match event {
        0x80 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::NoteOff { key: data1 }),
        )),
        0x90 => {
            if !ignore_range.contains(&data2) {
                renderer.send_event(SynthEvent::Channel(
                    channel,
                    ChannelEvent::Audio(ChannelAudioEvent::NoteOn {
                        key: data1,
                        vel: data2,
                    }),
                ));
            }
        }
        0xB0 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::Control(ControlEvent::Raw(data1, data2))),
        )),
        0xC0 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::ProgramChange(data1)),
        )),
        0xE0 => {
            let raw = (data1 as i16) | ((data2 as i16) << 7);
            let pitch = raw - 8192;
            renderer.send_event(SynthEvent::Channel(
                channel,
                ChannelEvent::Audio(ChannelAudioEvent::Control(ControlEvent::PitchBendValue(
                    pitch as f32 / 8192.0,
                ))),
            ));
        }
        _ => {}
    }
}
