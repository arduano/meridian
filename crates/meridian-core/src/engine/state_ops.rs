use std::time::Instant;

use crate::{
    error::MeridianError,
    midi::backend::MIDIFileBase,
    protocol::{CoreErrorCode, CoreEvent, FrameStats, RenderedFrame, StateSnapshot},
    render::{SceneConfig, pfa::wgpu::save_scene_headless, project_scene},
};

use super::{
    core_state::CoreState,
    support::{error_event, event_to_error},
};

impl CoreState {
    pub(super) fn render_frame(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<RenderedFrame, MeridianError> {
        self.sync_time();
        self.apply_viewport_overrides(viewport_width, viewport_height)?;
        self.validate_layout()
            .map_err(|event| event_to_error("invalid layout", &event))?;
        let scene = self
            .project_current_scene()
            .map_err(MeridianError::InvalidMidi)?;
        let stats = FrameStats::from_scene(&scene);

        Ok(RenderedFrame {
            state: self.snapshot(),
            layout: self.layout.clone(),
            stats,
            scene,
        })
    }

    pub(super) fn snapshot_after_layout_validation(&self) -> Vec<CoreEvent> {
        match self.validate_layout() {
            Ok(()) => vec![CoreEvent::StateSnapshot {
                state: self.snapshot(),
            }],
            Err(event) => vec![event],
        }
    }

    pub(super) fn apply_viewport_overrides(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<(), MeridianError> {
        if let Some(viewport_width) = viewport_width {
            if viewport_width == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_width must be > 0".into(),
                ));
            }
            self.layout.viewport_width = viewport_width;
        }
        if let Some(viewport_height) = viewport_height {
            if viewport_height == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_height must be > 0".into(),
                ));
            }
            self.layout.viewport_height = viewport_height;
        }
        Ok(())
    }

    pub(super) fn project_current_scene(
        &mut self,
    ) -> Result<crate::render::ProjectedScene, String> {
        let midi = self
            .midi
            .as_mut()
            .ok_or_else(|| "no midi loaded".to_string())?;
        Ok(project_scene(midi, self.current_time, &self.layout))
    }

    pub(super) fn midi_length(&self) -> f64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.midi_length())
            .unwrap_or(0.0)
    }

    pub(super) fn total_notes(&self) -> u64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.stats().total_notes)
            .unwrap_or(0)
    }

    pub(super) fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            midi_path: self.midi_path.clone(),
            midi_loaded: self.midi.is_some(),
            scene: self.layout.scene.clone(),
            current_time: self.current_time,
            playing: self.playing,
            midi_length: self.midi_length(),
            total_notes: self.total_notes(),
            view_range: self.layout.view_range,
            first_key: self.layout.first_key,
            last_key: self.layout.last_key,
            viewport_width: self.layout.viewport_width,
            viewport_height: self.layout.viewport_height,
        }
    }

    pub(super) fn sync_time(&mut self) {
        let now = Instant::now();
        if self.playing {
            if let Some(last_tick) = self.last_tick {
                self.current_time += now.duration_since(last_tick).as_secs_f64();
                self.current_time = self.current_time.min(self.midi_length().max(0.0));
                if self.current_time >= self.midi_length() && self.midi_length() > 0.0 {
                    self.playing = false;
                }
            }
        }
        self.last_tick = Some(now);
    }

    pub(super) fn validate_layout(&self) -> Result<(), CoreEvent> {
        if self.layout.viewport_width == 0 || self.layout.viewport_height == 0 {
            return Err(error_event(
                CoreErrorCode::InvalidViewport,
                "viewport dimensions must be greater than zero",
            ));
        }
        if self.layout.first_key > self.layout.last_key {
            return Err(error_event(
                CoreErrorCode::InvalidLayout,
                "first_key must be <= last_key",
            ));
        }
        if matches!(self.layout.scene, SceneConfig::ThreeD(_)) {
            return Err(error_event(
                CoreErrorCode::InvalidLayout,
                "3d scene config is reserved but not implemented yet",
            ));
        }
        Ok(())
    }

    pub(super) fn save_frame_headless(
        &mut self,
        output: std::path::PathBuf,
        format: crate::protocol::ImageOutputFormat,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Vec<CoreEvent> {
        match self.render_frame(viewport_width, viewport_height) {
            Ok(frame) => match save_scene_headless(
                frame.layout.viewport_width,
                frame.layout.viewport_height,
                &frame.scene,
                format,
                &output,
            ) {
                Ok(bytes_written) => vec![CoreEvent::FrameSaved {
                    output,
                    format,
                    state: frame.state,
                    stats: frame.stats,
                    bytes_written,
                }],
                Err(error) => vec![error_event(super::error_code(&error), error.to_string())],
            },
            Err(error) => vec![error_event(super::error_code(&error), error.to_string())],
        }
    }

    pub(super) fn broadcast(&self, event: CoreEvent) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|sender| sender.send(event.clone()).is_ok());
        }
    }
}
