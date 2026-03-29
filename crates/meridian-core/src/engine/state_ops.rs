use std::time::Instant;

use crate::{
    display::DisplayFrame,
    error::MeridianError,
    protocol::{CoreEvent, RenderedFrame, StateSnapshot},
};

use super::{
    core_state::CoreState,
    support::{error_event, event_to_error},
};

impl CoreState {
    pub(super) fn refresh_note_colors(&mut self) -> Result<(), MeridianError> {
        self.display.refresh_note_colors()
    }

    pub(super) fn render_frame(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<RenderedFrame, MeridianError> {
        self.sync_time();
        let transport = self.transport.snapshot();
        let frame = self
            .display
            .render_frame(transport, viewport_width, viewport_height)
            .map_err(|error| {
                if matches!(error, MeridianError::InvalidMidi(_)) {
                    if let Err(event) = self.display.validate_layout() {
                        return event_to_error("invalid layout", &event);
                    }
                }
                error
            })?;
        let DisplayFrame {
            layout,
            stats,
            scene,
        } = frame;

        Ok(RenderedFrame {
            state: self.snapshot(),
            layout,
            stats,
            scene,
        })
    }

    pub(super) fn snapshot_after_layout_validation(&self) -> Vec<CoreEvent> {
        match self.display.validate_layout() {
            Ok(()) => vec![CoreEvent::StateSnapshot {
                state: self.snapshot(),
            }],
            Err(event) => vec![event],
        }
    }

    pub(super) fn midi_length(&self) -> f64 {
        self.display.midi_length()
    }

    pub(super) fn total_notes(&self) -> u64 {
        self.display.total_notes()
    }

    pub(super) fn snapshot(&self) -> StateSnapshot {
        let transport = self.transport.snapshot();
        StateSnapshot {
            active_parsed_midi_id: self.active_parsed_midi_id,
            active_display_cache_id: self.active_display_cache_id,
            active_audio_cache_id: self.active_audio_cache_id,
            active_display_session_id: self.active_display_session_id,
            active_audio_session_id: self.active_audio_session_id,
            active_video_render_job_id: self.active_video_render_job_id,
            active_audio_render_job_id: self.active_audio_render_job_id,
            midi_path: self.midi_path.clone(),
            midi_loaded: self.display.midi_loaded(),
            audio: self.audio_config.clone(),
            audio_status: self.audio_player.status(),
            scene: self.display.scene().clone(),
            current_time: transport.current_time,
            playing: transport.playing,
            midi_length: self.midi_length(),
            total_notes: self.total_notes(),
            view_range: self.display.layout().view_range,
            first_key: self.display.layout().first_key,
            last_key: self.display.layout().last_key,
            viewport_width: self.display.layout().viewport_width,
            viewport_height: self.display.layout().viewport_height,
        }
    }

    pub(super) fn sync_time(&mut self) {
        let sync = self.transport.sync(self.midi_length(), Instant::now());
        if sync.stopped_at_end {
            self.audio_clock.set_time(sync.snapshot.current_time);
            self.audio_clock.set_playing(false);
        }
    }

    pub(super) fn tick_projector_physics(
        &mut self,
        delta_seconds: f64,
    ) -> Result<(), MeridianError> {
        self.display
            .tick_projector_physics(self.transport.current_time(), delta_seconds)
    }

    pub(super) fn save_frame_headless(
        &mut self,
        output: std::path::PathBuf,
        format: crate::protocol::ImageOutputFormat,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Vec<CoreEvent> {
        match self.render_frame(viewport_width, viewport_height) {
            Ok(frame) => match self.display.save_frame_headless(
                DisplayFrame {
                    layout: frame.layout,
                    stats: frame.stats,
                    scene: frame.scene,
                },
                output,
                format,
                frame.state,
            ) {
                Ok(event) => vec![event],
                Err(error) => vec![super::support::error_event(
                    super::error_code(&error),
                    error.to_string(),
                )],
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
