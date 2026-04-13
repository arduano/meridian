//! Video render configuration and time-range resolution.
//!
//! This module owns the validation boundary for render requests. The actual
//! render orchestration lives in `video/mod.rs`, but the rules about which
//! values are accepted belong here.

use std::sync::Arc;

use crate::{
    MeridianError, audio::AudioConfig, midi::audio_cache::InRamAudioCache,
    protocol::VideoRenderConfig, transport::PREVIEW_START_TIME_SECONDS,
};

pub struct VideoRenderAudioInputs {
    pub audio_cache: Arc<InRamAudioCache>,
    pub audio_config: AudioConfig,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedVideoTimeRange {
    pub start_time: f64,
    pub end_time: f64,
    pub duration_seconds: f64,
}

pub(crate) fn should_use_isolated_core(config: &VideoRenderConfig) -> bool {
    config.midi_path.is_some()
}

impl VideoRenderConfig {
    // Validation stays with the config type so callers can reject bad requests
    // before any render state is mutated.
    pub fn validate(&self) -> Result<(), MeridianError> {
        if self.fps <= 0.0 {
            return Err(MeridianError::InvalidMidi("fps must be > 0".into()));
        }
        if self.width == 0 || self.height == 0 {
            return Err(MeridianError::InvalidMidi(
                "video width and height must be > 0".into(),
            ));
        }
        if !self.container.matches_path(&self.output) {
            return Err(MeridianError::InvalidMidi(format!(
                "video output path must use .{} for the selected container",
                self.container.extension()
            )));
        }
        validate_time_bound(self.start_time, "video start time")?;
        validate_time_bound(self.end_time, "video end time")?;
        if let (Some(start_time), Some(end_time)) = (self.start_time, self.end_time) {
            if end_time <= start_time {
                return Err(MeridianError::InvalidMidi(
                    "video end time must be greater than start time".into(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn resolve_time_range(
        &self,
        midi_length: f64,
    ) -> Result<ResolvedVideoTimeRange, MeridianError> {
        let song_end = midi_length.max(0.0);
        let start_time = self.start_time.unwrap_or(PREVIEW_START_TIME_SECONDS);
        let end_time = self.end_time.unwrap_or(song_end);

        if start_time > song_end {
            return Err(MeridianError::InvalidMidi(format!(
                "video start time {start_time:.3} exceeds midi length {song_end:.3}"
            )));
        }
        if end_time > song_end {
            return Err(MeridianError::InvalidMidi(format!(
                "video end time {end_time:.3} exceeds midi length {song_end:.3}"
            )));
        }
        if end_time < start_time {
            return Err(MeridianError::InvalidMidi(
                "video end time must be greater than or equal to start time".into(),
            ));
        }

        Ok(ResolvedVideoTimeRange {
            start_time,
            end_time,
            duration_seconds: (end_time - start_time).max(0.0),
        })
    }
}

fn validate_time_bound(value: Option<f64>, label: &str) -> Result<(), MeridianError> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() {
        return Err(MeridianError::InvalidMidi(format!(
            "{label} must be a finite value"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{PREVIEW_START_TIME_SECONDS, ResolvedVideoTimeRange, should_use_isolated_core};
    use crate::protocol::{VideoOutputContainer, VideoRenderConfig};

    fn config(midi_path: Option<PathBuf>) -> VideoRenderConfig {
        VideoRenderConfig {
            midi_path,
            output: PathBuf::from("out.mp4"),
            container: VideoOutputContainer::Mp4,
            fps: 30.0,
            width: 1920,
            height: 1080,
            scene: None,
            view_range: None,
            time_space: None,
            start_time: None,
            end_time: None,
            first_key: None,
            last_key: None,
            ffmpeg_args: Vec::new(),
            export: Default::default(),
            audio: None,
        }
    }

    #[test]
    fn file_based_video_renders_use_an_isolated_core() {
        assert!(should_use_isolated_core(&config(Some(PathBuf::from(
            "clip.mid"
        )))));
    }

    #[test]
    fn live_context_video_renders_stay_on_the_active_core() {
        assert!(!should_use_isolated_core(&config(None)));
    }

    #[test]
    fn validate_allows_negative_start_time() {
        let mut config = config(None);
        config.start_time = Some(-0.5);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_end_time_before_start_time() {
        let mut config = config(None);
        config.start_time = Some(2.0);
        config.end_time = Some(1.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn resolve_time_range_defaults_to_full_song() {
        let resolved = config(None)
            .resolve_time_range(12.0)
            .expect("resolve range");
        assert_eq!(
            resolved,
            ResolvedVideoTimeRange {
                start_time: PREVIEW_START_TIME_SECONDS,
                end_time: 12.0,
                duration_seconds: 13.0,
            }
        );
    }

    #[test]
    fn resolve_time_range_uses_custom_bounds() {
        let mut config = config(None);
        config.start_time = Some(1.25);
        config.end_time = Some(3.5);
        let resolved = config.resolve_time_range(5.0).expect("resolve range");
        assert_eq!(
            resolved,
            ResolvedVideoTimeRange {
                start_time: 1.25,
                end_time: 3.5,
                duration_seconds: 2.25,
            }
        );
    }

    #[test]
    fn resolve_time_range_preserves_negative_start_bounds() {
        let mut config = config(None);
        config.start_time = Some(-1.0);
        config.end_time = Some(0.5);
        let resolved = config.resolve_time_range(5.0).expect("resolve range");
        assert_eq!(
            resolved,
            ResolvedVideoTimeRange {
                start_time: -1.0,
                end_time: 0.5,
                duration_seconds: 1.5,
            }
        );
    }
}
