use cpal::{
    SampleRate, SupportedStreamConfig,
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
        let synth =
            std::panic::catch_unwind(|| open_default_output_synth(config)).map_err(|_| {
                MeridianError::MidiLoad("xsynth panicked during output initialization".into())
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

fn open_default_output_synth(config: &AudioConfig) -> Result<RealtimeSynth, MeridianError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| MeridianError::Platform("no default output device available".into()))?;

    let stream_config = select_output_config(&device, config.xsynth.render.audio_params)
        .or_else(|_| default_output_config(&device))?;

    RealtimeSynth::open(config.xsynth.config.clone(), &device, stream_config)
        .map_err(|e| MeridianError::Platform(format!("failed to open xsynth output: {e}")))
}

fn select_output_config(
    device: &cpal::Device,
    desired: AudioStreamParams,
) -> Result<SupportedStreamConfig, MeridianError> {
    let mut configs = device
        .supported_output_configs()
        .map_err(|e| MeridianError::Platform(format!("failed to enumerate output configs: {e}")))?;

    configs
        .find(|config| {
            config.channels() == desired.channels.count()
                && config.min_sample_rate().0 <= desired.sample_rate
                && desired.sample_rate <= config.max_sample_rate().0
        })
        .map(|config| config.with_sample_rate(SampleRate(desired.sample_rate)))
        .ok_or_else(|| {
            MeridianError::Unsupported(format!(
                "default output device does not support {} Hz {} output",
                desired.sample_rate,
                match desired.channels {
                    xsynth_core::ChannelCount::Mono => "mono",
                    xsynth_core::ChannelCount::Stereo => "stereo",
                }
            ))
        })
}

fn default_output_config(device: &cpal::Device) -> Result<SupportedStreamConfig, MeridianError> {
    let config = device.default_output_config().map_err(|e| {
        MeridianError::Platform(format!("failed to query default output config: {e}"))
    })?;

    if xsynth_core::ChannelCount::from_count(config.channels()).is_none() {
        return Err(MeridianError::Unsupported(format!(
            "default output device uses unsupported channel count {}",
            config.channels()
        )));
    }

    Ok(config)
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
