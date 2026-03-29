use std::{
    ops::RangeInclusive,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
use xsynth_core::{
    AudioStreamParams, ChannelCount,
    channel::{ChannelAudioEvent, ChannelEvent, ControlEvent},
    channel_group::{ChannelGroupConfig, SynthEvent},
};
use xsynth_render::{OfflineRenderConfig, OfflineWavRenderer};

use crate::{MeridianError, midi::MidiCacheStack};

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
        }
    }
}

pub fn render_audio(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &AudioRenderConfig,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_audio_inner(
            midi_cache,
            audio_config,
            soundfont_cache,
            config,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioRenderEvent {
    RenderStarted {
        output: PathBuf,
        sample_rate: u32,
        channels: u16,
        total_events: usize,
    },
    RenderProgress {
        event_index: usize,
        total_events: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
        voice_count: u64,
    },
    RenderFinished {
        output: PathBuf,
        frames_written: u64,
        rendered_seconds: f64,
    },
    RenderCancelled {
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
        &cancel,
        callback,
    )
}

fn render_audio_inner(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
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
    let events = midi_cache.audio_cache()?;

    callback(AudioRenderEvent::RenderStarted {
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
        output: render_config.output.clone(),
        frames_written,
        rendered_seconds: current_time,
    });

    Ok(())
}

struct OfflineAudioRenderer {
    inner: OfflineWavRenderer,
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
        let render_config = OfflineRenderConfig {
            group_options,
            use_limiter,
        };
        let inner =
            OfflineWavRenderer::new(render_config, output, soundfonts, layers).map_err(|e| {
                MeridianError::Platform(format!("failed to create xsynth wav renderer: {e}"))
            })?;
        Ok(Self {
            inner,
            frames_written: 0,
        })
    }

    fn render_batch(&mut self, seconds: f64) -> Result<(), MeridianError> {
        self.inner
            .render_batch(seconds)
            .map_err(|e| MeridianError::Platform(format!("xsynth render batch failed: {e}")))?;
        self.frames_written = self.inner.frames_written();
        Ok(())
    }

    fn finalize(self) -> Result<u64, MeridianError> {
        self.inner.finalize().map_err(|e| {
            MeridianError::Platform(format!("failed to finalize xsynth wav render: {e}"))
        })
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
