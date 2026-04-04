use std::sync::{Arc, RwLock};

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use xsynth_core::{AudioStreamParams, ChannelCount};

use crate::error::MeridianError;

use super::{
    config::{AudioBackend, AudioConfig},
    soundfont_cache::SoundfontCache,
    xsynth::XSynthPlayer,
};

pub trait MidiAudioPlayer: Send + Sync {
    fn push_event(&mut self, data: u32);
    fn reset(&mut self);
    fn voice_count(&self) -> Option<u64>;
    fn stream_params(&self) -> Option<AudioStreamParams>;
    fn configure(
        &mut self,
        config: &AudioConfig,
        cache: &SoundfontCache,
    ) -> Result<(), MeridianError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioStatus {
    pub backend: AudioBackend,
    pub voice_count: Option<u64>,
    pub stream_params: Option<AudioStreamParams>,
    pub loaded_soundfonts: usize,
    pub active: bool,
    pub supports_44100_hz: bool,
    pub supports_48000_hz: bool,
    pub supports_88200_hz: bool,
    pub supports_96000_hz: bool,
    pub supports_176400_hz: bool,
    pub supports_192000_hz: bool,
    pub supports_mono: bool,
    pub supports_stereo: bool,
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self {
            backend: AudioBackend::None,
            voice_count: None,
            stream_params: None,
            loaded_soundfonts: 0,
            active: false,
            supports_44100_hz: false,
            supports_48000_hz: false,
            supports_88200_hz: false,
            supports_96000_hz: false,
            supports_176400_hz: false,
            supports_192000_hz: false,
            supports_mono: false,
            supports_stereo: false,
        }
    }
}

#[derive(Default)]
struct OutputSupport {
    supports_44100_hz: bool,
    supports_48000_hz: bool,
    supports_88200_hz: bool,
    supports_96000_hz: bool,
    supports_176400_hz: bool,
    supports_192000_hz: bool,
    supports_mono: bool,
    supports_stereo: bool,
}

struct EmptyPlayer;

impl MidiAudioPlayer for EmptyPlayer {
    fn push_event(&mut self, _data: u32) {}
    fn reset(&mut self) {}
    fn voice_count(&self) -> Option<u64> {
        None
    }
    fn stream_params(&self) -> Option<AudioStreamParams> {
        None
    }
    fn configure(
        &mut self,
        _config: &AudioConfig,
        _cache: &SoundfontCache,
    ) -> Result<(), MeridianError> {
        Ok(())
    }
}

pub struct MeridianAudioPlayer {
    player: RwLock<Box<dyn MidiAudioPlayer>>,
    status: RwLock<AudioStatus>,
    cache: SoundfontCache,
}

impl MeridianAudioPlayer {
    pub fn new(config: &AudioConfig) -> Arc<Self> {
        let this = Arc::new(Self {
            player: RwLock::new(Box::new(EmptyPlayer)),
            status: RwLock::new(AudioStatus::default()),
            cache: SoundfontCache::new(),
        });
        let _ = this.switch(config);
        this
    }

    pub fn switch(&self, config: &AudioConfig) -> Result<(), MeridianError> {
        let mut new_player: Box<dyn MidiAudioPlayer> = match config.backend {
            AudioBackend::None => Box::new(EmptyPlayer),
            AudioBackend::Xsynth => Box::new(XSynthPlayer::new(config)?),
        };
        let output_support = match config.backend {
            AudioBackend::None => OutputSupport::default(),
            AudioBackend::Xsynth => default_output_support(),
        };
        new_player.configure(config, &self.cache)?;
        let status = AudioStatus {
            backend: config.backend,
            voice_count: new_player.voice_count(),
            stream_params: new_player.stream_params(),
            loaded_soundfonts: config.soundfonts.iter().filter(|sf| sf.enabled).count(),
            active: !matches!(config.backend, AudioBackend::None),
            supports_44100_hz: output_support.supports_44100_hz,
            supports_48000_hz: output_support.supports_48000_hz,
            supports_88200_hz: output_support.supports_88200_hz,
            supports_96000_hz: output_support.supports_96000_hz,
            supports_176400_hz: output_support.supports_176400_hz,
            supports_192000_hz: output_support.supports_192000_hz,
            supports_mono: output_support.supports_mono,
            supports_stereo: output_support.supports_stereo,
        };
        *write_lock(&self.player) = new_player;
        *write_lock(&self.status) = status;
        Ok(())
    }

    pub fn push_events(&self, data: impl Iterator<Item = u32>) {
        let mut player = write_lock(&self.player);
        for event in data {
            player.push_event(event);
        }
        write_lock(&self.status).voice_count = player.voice_count();
    }

    pub fn reset(&self) {
        let mut player = write_lock(&self.player);
        player.reset();
        write_lock(&self.status).voice_count = player.voice_count();
    }

    pub fn status(&self) -> AudioStatus {
        let mut status = read_lock(&self.status).clone();
        let player = read_lock(&self.player);
        status.voice_count = player.voice_count();
        status.stream_params = player.stream_params();
        status
    }
}

fn default_output_support() -> OutputSupport {
    let Some(device) = cpal::default_host().default_output_device() else {
        return OutputSupport::default();
    };
    let Ok(configs) = device.supported_output_configs() else {
        return OutputSupport::default();
    };

    let mut support = OutputSupport::default();
    for config in configs {
        match ChannelCount::from_count(config.channels()) {
            Some(ChannelCount::Mono) => support.supports_mono = true,
            Some(ChannelCount::Stereo) => support.supports_stereo = true,
            None => {}
        }

        for sample_rate in [44_100, 48_000, 88_200, 96_000, 176_400, 192_000] {
            if config.min_sample_rate().0 <= sample_rate
                && sample_rate <= config.max_sample_rate().0
            {
                match sample_rate {
                    44_100 => support.supports_44100_hz = true,
                    48_000 => support.supports_48000_hz = true,
                    88_200 => support.supports_88200_hz = true,
                    96_000 => support.supports_96000_hz = true,
                    176_400 => support.supports_176400_hz = true,
                    192_000 => support.supports_192000_hz = true,
                    _ => {}
                }
            }
        }
    }

    support
}

fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    match lock.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    match lock.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
