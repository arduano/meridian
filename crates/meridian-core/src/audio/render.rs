use std::{
    fs::File,
    io::BufWriter,
    ops::RangeInclusive,
    path::PathBuf,
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
    protocol::AudioRenderJobId,
};

use super::{AudioBackend, AudioConfig, soundfont_cache::SoundfontCache};

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
            soundfonts: Vec::new(),
        }
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

    let sample_rate = render_config
        .sample_rate
        .unwrap_or(audio_config.xsynth.render.audio_params.sample_rate);
    let channels = render_config
        .channels
        .unwrap_or(audio_config.xsynth.render.audio_params.channels.count());
    let use_limiter = render_config
        .use_limiter
        .unwrap_or(audio_config.xsynth.render.use_limiter);

    let channel_count = ChannelCount::from_count(channels).ok_or_else(|| {
        MeridianError::Platform(format!(
            "unsupported channel count {}; only 1 or 2 are supported",
            channels
        ))
    })?;
    let audio_params = AudioStreamParams::new(sample_rate, channel_count);
    callback(AudioRenderEvent::RenderStarted {
        job_id,
        output: render_config.output.clone(),
        sample_rate,
        channels,
        total_events: events.events().len(),
    });

    let mut renderer = OfflineAudioRenderer::new(
        audio_config,
        soundfont_cache,
        render_config.output.clone(),
        audio_params,
        use_limiter,
    )?;

    let mut current_time = 0.0;
    let total_events = events.events().len();
    let progress_stride = progress_stride(total_events);
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
            return Ok(());
        }

        let delta = (event.time - current_time).max(0.0);
        if delta > 0.0 {
            renderer.render_batch(delta)?;
            current_time = event.time;
        }

        for packed in event.iter_events() {
            dispatch_packed_event(
                &mut renderer,
                packed,
                &audio_config.xsynth.config.ignore_range,
            );
        }

        if event_index == 0 || event_index + 1 == total_events || event_index % progress_stride == 0
        {
            callback(AudioRenderEvent::RenderProgress {
                job_id,
                event_index: event_index + 1,
                total_events,
                time_seconds: event.time,
                rendered_seconds: current_time,
                frames_written: renderer.frames_written(),
                voice_count: renderer.voice_count(),
            });
        }
    }

    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::AllNotesOff,
    )));
    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::ResetControl,
    )));
    let frames_written = renderer.finalize()?;

    callback(AudioRenderEvent::RenderFinished {
        job_id,
        output: render_config.output.clone(),
        frames_written,
        rendered_seconds: current_time,
    });

    Ok(())
}

struct OfflineAudioRenderer {
    inner: ChannelGroup,
    writer: WavWriter<BufWriter<File>>,
    limiter: Option<VolumeLimiter>,
    scratch: Vec<f32>,
    missed_samples: f64,
    channel_count: usize,
    frames_written: u64,
}

impl OfflineAudioRenderer {
    fn new(
        audio_config: &AudioConfig,
        soundfont_cache: &SoundfontCache,
        output: PathBuf,
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
        let mut inner = ChannelGroup::new(group_options);
        inner.send_event(SynthEvent::AllChannels(ChannelEvent::Config(
            ChannelConfigEvent::SetSoundfonts(soundfonts),
        )));
        inner.send_event(SynthEvent::AllChannels(ChannelEvent::Config(
            ChannelConfigEvent::SetLayerCount(layers),
        )));

        let writer = WavWriter::create(
            output,
            WavSpec {
                channels: audio_params.channels.count(),
                sample_rate: audio_params.sample_rate,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            },
        )
        .map_err(|error| {
            MeridianError::Platform(format!("failed to create output wav file: {error}"))
        })?;
        Ok(Self {
            inner,
            writer,
            limiter: use_limiter.then(|| VolumeLimiter::new(audio_params.channels.count())),
            scratch: Vec::new(),
            missed_samples: 0.0,
            channel_count: audio_params.channels.count() as usize,
            frames_written: 0,
        })
    }

    fn render_batch(&mut self, seconds: f64) -> Result<(), MeridianError> {
        if seconds > 10.0 {
            let mut remaining = seconds;
            while remaining > 10.0 {
                self.render_batch(10.0)?;
                remaining -= 10.0;
            }
            if remaining > 0.0 {
                self.render_batch(remaining)?;
            }
            return Ok(());
        }

        let samples = self.stream_sample_count(seconds);
        if samples == 0 {
            return Ok(());
        }

        self.scratch.resize(samples, 0.0);
        self.inner.read_samples(&mut self.scratch);
        self.write_buffer()?;
        Ok(())
    }

    fn finalize(mut self) -> Result<u64, MeridianError> {
        loop {
            self.scratch
                .resize(self.channel_count * self.output_sample_rate() as usize, 0.0);
            self.inner.read_samples(&mut self.scratch);

            if let Some(limiter) = &mut self.limiter {
                limiter.limit(&mut self.scratch);
            }

            if self
                .scratch
                .iter()
                .all(|sample| (-0.0001..=0.0001).contains(sample))
            {
                break;
            }

            self.write_samples_from_scratch()?;
        }

        self.writer.finalize().map_err(|error| {
            MeridianError::Platform(format!("failed to finalize wav output: {error}"))
        })?;
        Ok(self.frames_written)
    }

    fn send_event(&mut self, event: SynthEvent) {
        self.inner.send_event(event);
    }

    fn voice_count(&self) -> u64 {
        self.inner.voice_count()
    }

    fn frames_written(&self) -> u64 {
        self.frames_written
    }

    fn stream_sample_count(&mut self, seconds: f64) -> usize {
        let sample_frames = self.output_sample_rate() as f64 * seconds + self.missed_samples;
        self.missed_samples = sample_frames.fract();
        sample_frames.floor() as usize * self.channel_count
    }

    fn output_sample_rate(&self) -> u32 {
        self.inner.stream_params().sample_rate
    }

    fn write_buffer(&mut self) -> Result<(), MeridianError> {
        if let Some(limiter) = &mut self.limiter {
            limiter.limit(&mut self.scratch);
        }
        self.write_samples_from_scratch()
    }

    fn write_samples_from_scratch(&mut self) -> Result<(), MeridianError> {
        for &sample in &self.scratch {
            self.writer.write_sample(sample).map_err(|error| {
                MeridianError::Platform(format!("failed to write wav sample: {error}"))
            })?;
        }
        self.frames_written += (self.scratch.len() / self.channel_count) as u64;
        Ok(())
    }
}

fn progress_stride(total_events: usize) -> usize {
    (total_events / 200).max(1)
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
