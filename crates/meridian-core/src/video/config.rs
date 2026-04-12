use std::sync::Arc;

use crate::{
    MeridianError,
    audio::AudioConfig,
    midi::audio_cache::InRamAudioCache,
    protocol::VideoRenderConfig,
};

pub struct VideoRenderAudioInputs {
    pub audio_cache: Arc<InRamAudioCache>,
    pub audio_config: AudioConfig,
}

pub(crate) fn should_use_isolated_core(config: &VideoRenderConfig) -> bool {
    config.midi_path.is_some()
}

impl VideoRenderConfig {
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::should_use_isolated_core;
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
            first_key: None,
            last_key: None,
            ffmpeg_args: Vec::new(),
            export: Default::default(),
            audio: None,
        }
    }

    #[test]
    fn file_based_video_renders_use_an_isolated_core() {
        assert!(should_use_isolated_core(&config(Some(PathBuf::from("clip.mid")))));
    }

    #[test]
    fn live_context_video_renders_stay_on_the_active_core() {
        assert!(!should_use_isolated_core(&config(None)));
    }
}
