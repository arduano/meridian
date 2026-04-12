use std::path::Path;

use xsynth_core::{
    AudioPipe, AudioStreamParams,
    channel::{ChannelConfigEvent, ChannelEvent},
    channel_group::{ChannelGroup, ChannelGroupConfig, SynthEvent},
    effects::VolumeLimiter,
};

use crate::{
    MeridianError,
    audio::{AudioConfig, soundfont_cache::SoundfontCache},
};

use super::writer::AudioSampleWriter;

pub(crate) struct OfflineAudioRenderer {
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
    pub(crate) fn new_wav(
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

    pub(crate) fn new_raw_pipe(
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

    pub(crate) fn render_batch(&mut self, seconds: f64) -> Result<(), MeridianError> {
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

    pub(crate) fn finalize(mut self) -> Result<u64, MeridianError> {
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

    pub(crate) fn send_event(&mut self, event: SynthEvent) {
        self.channel_group.send_event(event);
    }

    pub(crate) fn voice_count(&self) -> u64 {
        self.channel_group.voice_count()
    }

    pub(crate) fn frames_written(&self) -> u64 {
        self.frames_written
    }
}
