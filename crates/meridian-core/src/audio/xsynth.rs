use std::ops::{Deref, DerefMut};

use xsynth_core::{
    AudioStreamParams,
    channel::{ChannelConfigEvent, ChannelEvent},
};
use xsynth_realtime::{RealtimeEventSender, RealtimeSynth, RealtimeSynthStatsReader, SynthEvent};

use crate::error::MeridianError;

use super::{config::AudioConfig, player::MidiAudioPlayer, soundfont_cache::SoundfontCache};

#[repr(transparent)]
struct SendSyncSynth<T>(T);

// TODO: remove this wrapper once xsynth exposes a sound thread handle that is natively Send + Sync.
// This mirrors Wasabi's workaround, but the real fix belongs upstream in xsynth.
unsafe impl<T> Send for SendSyncSynth<T> {}
unsafe impl<T> Sync for SendSyncSynth<T> {}

impl<T> Deref for SendSyncSynth<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SendSyncSynth<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct XSynthPlayer {
    sender: RealtimeEventSender,
    stats: RealtimeSynthStatsReader,
    stream_params: AudioStreamParams,
    synth: SendSyncSynth<RealtimeSynth>,
}

impl XSynthPlayer {
    pub fn new(config: &AudioConfig) -> Result<Self, MeridianError> {
        let synth = std::panic::catch_unwind(|| {
            SendSyncSynth(RealtimeSynth::open_with_default_output(
                config.xsynth.config.clone(),
            ))
        })
        .map_err(|_| {
            MeridianError::MidiLoad("xsynth panicked during output initialization".into())
        })?;
        let sender = synth.get_sender_ref().clone();
        let stats = synth.get_stats();
        let stream_params = synth.stream_params();
        Ok(Self {
            sender,
            stats,
            stream_params,
            synth,
        })
    }
}

impl MidiAudioPlayer for XSynthPlayer {
    fn push_event(&mut self, data: u32) {
        self.sender.send_event_u32(data);
    }

    fn reset(&mut self) {
        self.sender.reset_synth();
    }

    fn voice_count(&self) -> Option<u64> {
        Some(self.stats.voice_count())
    }

    fn stream_params(&self) -> Option<AudioStreamParams> {
        Some(self.stream_params)
    }

    fn configure(
        &mut self,
        config: &AudioConfig,
        cache: &SoundfontCache,
    ) -> Result<(), MeridianError> {
        let layers = if config.xsynth.limit_layers {
            Some(config.xsynth.layers)
        } else {
            None
        };
        self.sender
            .send_event(SynthEvent::AllChannels(ChannelEvent::Config(
                ChannelConfigEvent::SetLayerCount(layers),
            )));
        self.synth.set_buffer(config.xsynth.config.render_window_ms);
        self.sender
            .set_ignore_range(config.xsynth.config.ignore_range.clone());
        let soundfonts = cache.load_enabled(&config.soundfonts, self.stream_params)?;
        let mut sender = self.sender.clone();
        sender.send_event(SynthEvent::AllChannels(ChannelEvent::Config(
            ChannelConfigEvent::SetSoundfonts(soundfonts),
        )));
        Ok(())
    }
}
