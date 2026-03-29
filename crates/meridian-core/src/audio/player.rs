use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use xsynth_core::AudioStreamParams;

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
    fn configure(&mut self, config: &AudioConfig, cache: &SoundfontCache) -> Result<(), MeridianError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioStatus {
    pub backend: AudioBackend,
    pub voice_count: Option<u64>,
    pub stream_params: Option<AudioStreamParams>,
    pub loaded_soundfonts: usize,
    pub active: bool,
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self {
            backend: AudioBackend::None,
            voice_count: None,
            stream_params: None,
            loaded_soundfonts: 0,
            active: false,
        }
    }
}

struct EmptyPlayer;

impl MidiAudioPlayer for EmptyPlayer {
    fn push_event(&mut self, _data: u32) {}
    fn reset(&mut self) {}
    fn voice_count(&self) -> Option<u64> { None }
    fn stream_params(&self) -> Option<AudioStreamParams> { None }
    fn configure(&mut self, _config: &AudioConfig, _cache: &SoundfontCache) -> Result<(), MeridianError> { Ok(()) }
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
        new_player.configure(config, &self.cache)?;
        let status = AudioStatus {
            backend: config.backend,
            voice_count: new_player.voice_count(),
            stream_params: new_player.stream_params(),
            loaded_soundfonts: config.soundfonts.iter().filter(|sf| sf.enabled).count(),
            active: !matches!(config.backend, AudioBackend::None),
        };
        *self.player.write().unwrap() = new_player;
        *self.status.write().unwrap() = status;
        Ok(())
    }

    pub fn push_events(&self, data: impl Iterator<Item = u32>) {
        let mut player = self.player.write().unwrap();
        for event in data {
            player.push_event(event);
        }
        self.status.write().unwrap().voice_count = player.voice_count();
    }

    pub fn reset(&self) {
        let mut player = self.player.write().unwrap();
        player.reset();
        self.status.write().unwrap().voice_count = player.voice_count();
    }

    pub fn status(&self) -> AudioStatus {
        let mut status = self.status.read().unwrap().clone();
        let player = self.player.read().unwrap();
        status.voice_count = player.voice_count();
        status.stream_params = player.stream_params();
        status
    }
}
