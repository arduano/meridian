use std::{path::Path, sync::Arc, time::Instant};

use crate::{
    engine::support::error_event,
    midi::MidiBuildProgress,
    protocol::{CoreErrorCode, CoreEvent, ParsedMidiId},
};

use super::super::core_state::CoreState;

impl CoreState {
    pub(super) fn midi_load_cancelled_events() -> Vec<CoreEvent> {
        vec![error_event(CoreErrorCode::Cancelled, "midi load cancelled")]
    }

    pub(super) fn midi_load_was_cancelled(&self, load_generation: u64) -> bool {
        self.core_handle.current_midi_load_generation() != load_generation
    }

    pub(super) fn ensure_midi_load_not_cancelled(
        &self,
        load_generation: u64,
    ) -> Result<(), Vec<CoreEvent>> {
        if self.midi_load_was_cancelled(load_generation) {
            Err(Self::midi_load_cancelled_events())
        } else {
            Ok(())
        }
    }

    pub(super) fn broadcast_midi_load_progress(
        &self,
        path: &Path,
        progress: MidiBuildProgress,
        status: &str,
    ) {
        self.broadcast(CoreEvent::MidiLoadProgress {
            path: path.to_path_buf(),
            progress,
            status: status.into(),
        });
    }

    fn active_midi_conflicts_with(&self, parsed_midi_id: ParsedMidiId) -> bool {
        self.active_parsed_midi_id
            .is_some_and(|active_parsed_midi_id| active_parsed_midi_id != parsed_midi_id)
    }

    pub(in crate::engine) fn cancel_active_render_jobs(
        &mut self,
        cancel_video: bool,
        cancel_audio: bool,
    ) -> Vec<CoreEvent> {
        let mut events = Vec::new();

        if cancel_video && self.request_cancel_render_video() {
            self.active_video_render_job_id = self.active_video_render_job_id_from_state();
            events.push(CoreEvent::VideoRenderStatus {
                status: self.video_render_status(),
            });
        }

        if cancel_audio && self.request_cancel_render_audio() {
            self.active_audio_render_job_id = self.active_audio_render_job_id_from_state();
            events.push(CoreEvent::AudioRenderStatus {
                status: self.audio_render_status(),
            });
        }

        events
    }

    pub(super) fn clear_active_midi_context(&mut self, now: Instant) {
        let _ = self.cancel_active_render_jobs(true, true);
        self.audio_session = None;
        self.processed_midi = None;
        self.midi_cache = None;
        self.current_audio_cache = None;
        self.active_parsed_midi_id = None;
        self.active_processed_midi_id = None;
        self.active_display_cache_id = None;
        self.active_audio_cache_id = None;
        self.active_display_session_id = None;
        self.active_audio_session_id = None;
        self.midi_path = None;
        self.transport.reset(now);
        self.audio_clock.set_time(0.0);
        self.audio_clock.set_playing(false);
        self.audio_player.reset();
        self.display.unload_midi(now);
    }

    pub(super) fn clear_conflicting_active_midi(&mut self, parsed_midi_id: ParsedMidiId) {
        if self.active_midi_conflicts_with(parsed_midi_id) {
            self.clear_active_midi_context(Instant::now());
        }
    }
}

pub(super) fn broadcast_progress_to_subscribers(
    subscribers: &Arc<std::sync::Mutex<Vec<flume::Sender<CoreEvent>>>>,
    path: &Path,
    progress: MidiBuildProgress,
    status: &str,
) {
    let event = CoreEvent::MidiLoadProgress {
        path: path.to_path_buf(),
        progress,
        status: status.into(),
    };
    if let Ok(mut subscribers) = subscribers.lock() {
        subscribers.retain(|sender| sender.send(event.clone()).is_ok());
    }
}

pub(super) fn scale_midi_build_progress(
    progress: MidiBuildProgress,
    progress_start: f32,
    progress_end: f32,
) -> MidiBuildProgress {
    let fraction_complete = progress.fraction_complete.map(|phase| {
        (progress_start + phase * (progress_end - progress_start))
            .clamp(progress_start, progress_end)
    });
    MidiBuildProgress {
        fraction_complete,
        ..progress
    }
}
