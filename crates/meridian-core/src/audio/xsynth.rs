use cpal::{
    SampleRate,
    traits::{DeviceTrait, HostTrait},
};
use xsynth_core::{
    AudioStreamParams,
    channel::{ChannelConfigEvent, ChannelEvent},
};
use xsynth_realtime::{RealtimeEventSender, RealtimeSynth, RealtimeSynthStatsReader, SynthEvent};

use crate::error::MeridianError;

use super::{config::AudioConfig, player::MidiAudioPlayer, soundfont_cache::SoundfontCache};

pub struct XSynthPlayer {
    sender: RealtimeEventSender,
    stats: RealtimeSynthStatsReader,
    stream_params: AudioStreamParams,
    synth: RealtimeSynth,
}

impl XSynthPlayer {
    pub fn new(config: &AudioConfig) -> Result<Self, MeridianError> {
        let synth = std::panic::catch_unwind(|| open_output_synth(config)).map_err(|_| {
            MeridianError::Platform("xsynth panicked during output initialization".into())
        })??;
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

fn open_output_synth(config: &AudioConfig) -> Result<RealtimeSynth, MeridianError> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or_else(|| {
        MeridianError::Platform("no default audio output device is available".into())
    })?;

    let desired = config.xsynth.render.audio_params;
    let stream_config = if let Ok(configs) = device.supported_output_configs() {
        configs
            .filter(|candidate| candidate.channels() == desired.channels.count())
            .find_map(|candidate| candidate.try_with_sample_rate(SampleRate(desired.sample_rate)))
    } else {
        None
    }
    .or_else(|| device.default_output_config().ok())
    .ok_or_else(|| {
        MeridianError::Platform("failed to determine an output stream configuration".into())
    })?;

    RealtimeSynth::open(config.xsynth.config.clone(), &device, stream_config).map_err(|error| {
        MeridianError::Platform(format!("xsynth output initialization failed: {error}"))
    })
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
