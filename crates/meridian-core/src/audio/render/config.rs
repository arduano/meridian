use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xsynth_core::{AudioStreamParams, ChannelCount};

use crate::{
    MeridianError,
    protocol::{AudioOutputFormat, VideoAudioConfig},
};

use super::AudioConfig;

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

impl AudioRenderConfig {
    pub fn validate(&self) -> Result<(), MeridianError> {
        if !self.format.matches_path(&self.output) {
            return Err(MeridianError::InvalidMidi(format!(
                "audio output path must use .{} for the selected format",
                self.format.extension()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedAudioRenderSettings {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) use_limiter: bool,
}

impl ResolvedAudioRenderSettings {
    pub(crate) fn audio_params(self) -> Result<AudioStreamParams, MeridianError> {
        validate_channel_count(self.channels)?;
        Ok(AudioStreamParams::new(
            self.sample_rate,
            ChannelCount::from_count(self.channels).expect("validated channel count"),
        ))
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
    validate_channel_count(channels)?;
    Ok((sample_rate, channels, use_limiter))
}

pub(crate) fn resolve_render_settings(
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

    validate_channel_count(channels)?;

    Ok(ResolvedAudioRenderSettings {
        sample_rate,
        channels,
        use_limiter,
    })
}

fn validate_channel_count(channels: u16) -> Result<(), MeridianError> {
    ChannelCount::from_count(channels).ok_or_else(|| {
        MeridianError::Platform(format!(
            "unsupported channel count {}; only 1 or 2 are supported",
            channels
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AudioRenderConfig;
    use crate::protocol::AudioOutputFormat;

    #[test]
    fn validate_rejects_mismatched_audio_output_extension() {
        let config = AudioRenderConfig {
            output: "render.wav".into(),
            format: AudioOutputFormat::Mp3,
            ..AudioRenderConfig::default()
        };

        let error = config.validate().expect_err("validation should fail");

        assert!(error.to_string().contains("audio output path"));
        assert!(error.to_string().contains(".mp3"));
    }

    #[test]
    fn validate_allows_matching_audio_output_extension() {
        let config = AudioRenderConfig {
            output: "render.flac".into(),
            format: AudioOutputFormat::Flac,
            ..AudioRenderConfig::default()
        };

        assert!(config.validate().is_ok());
    }
}
