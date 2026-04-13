use std::{path::PathBuf, sync::Arc, time::Instant};

use crate::{
    midi::MidiBuildProgress,
    protocol::{AudioCacheId, CoreErrorCode, CoreEvent, DisplayCacheId},
};

use super::super::core_state::CoreState;

impl CoreState {
    pub(super) fn prepare_display_session_for_midi(
        &self,
        midi: crate::midi::MIDIFileUnion,
        now: Instant,
    ) -> Result<crate::display::LiveDisplaySession, crate::error::MeridianError> {
        let layout = self.display.layout().clone();
        let mut display = crate::display::LiveDisplaySession::new();
        display.set_scene_config(layout.scene, now);
        display.set_view_range(layout.view_range, Some(layout.time_space));
        display.set_key_range(layout.first_key, layout.last_key);
        display
            .apply_viewport_overrides(Some(layout.viewport_width), Some(layout.viewport_height))?;
        display.load_midi(midi, now);
        display.refresh_note_colors()?;
        Ok(display)
    }

    pub(in crate::engine) fn attach_display_cache_resource(
        &mut self,
        display_cache_id: DisplayCacheId,
    ) -> Vec<CoreEvent> {
        let Some(parsed_midi_id) = self
            .display_caches
            .get(&display_cache_id)
            .map(|resource| resource.parsed_midi_id)
        else {
            return vec![crate::engine::support::error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown display_cache_id {}", display_cache_id.0),
            )];
        };
        let display_cache = Arc::clone(&self.display_caches[&display_cache_id].cache);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![crate::engine::support::error_event(
                CoreErrorCode::Internal,
                "display cache is missing its parsed MIDI parent",
            )];
        };
        let parsed_path = parsed.path.clone();
        let parsed_stack = parsed.cache_stack.clone();

        let now = Instant::now();
        let display = match self.prepare_display_session_for_midi(
            crate::midi::MIDIFileUnion::InRam(display_cache.instantiate()),
            now,
        ) {
            Ok(display) => display,
            Err(error) => {
                return vec![crate::engine::support::error_event(
                    CoreErrorCode::Internal,
                    error.to_string(),
                )];
            }
        };

        if self.active_midi_conflicts_with(parsed_midi_id) {
            self.clear_active_midi_context(now);
        }

        self.processed_midi = None;
        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_processed_midi_id = None;
        self.active_display_cache_id = Some(display_cache_id);
        self.active_display_session_id = None;
        self.midi_path = Some(parsed_path);
        self.display = display;
        self.transport.reset(now);
        self.display.mark_physics_tick(now);
        self.audio_clock.set_time(self.transport.current_time());
        self.audio_clock.set_playing(false);
        vec![CoreEvent::DisplayCacheAttached {
            display_cache_id,
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn attach_audio_cache_resource(
        &mut self,
        audio_cache_id: AudioCacheId,
    ) -> Vec<CoreEvent> {
        let Some(parsed_midi_id) = self
            .audio_caches
            .get(&audio_cache_id)
            .map(|resource| resource.parsed_midi_id)
        else {
            return vec![crate::engine::support::error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown audio_cache_id {}", audio_cache_id.0),
            )];
        };
        self.clear_conflicting_active_midi(parsed_midi_id);
        let audio_cache = Arc::clone(&self.audio_caches[&audio_cache_id].cache);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![crate::engine::support::error_event(
                CoreErrorCode::Internal,
                "audio cache is missing its parsed MIDI parent",
            )];
        };
        let parsed_stack = parsed.cache_stack.clone();

        self.processed_midi = None;
        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_processed_midi_id = None;
        self.active_audio_cache_id = Some(audio_cache_id);
        self.active_audio_session_id = None;
        self.midi_path = Some(parsed.path.clone());
        self.current_audio_cache = Some(audio_cache);
        let now = Instant::now();
        self.transport.reset(now);
        self.display.mark_physics_tick(now);
        self.audio_clock.set_time(self.transport.current_time());
        self.audio_clock.set_playing(false);
        self.restart_audio_session();
        vec![CoreEvent::AudioCacheAttached {
            audio_cache_id,
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn unload_display_context(&mut self) -> Vec<CoreEvent> {
        let now = Instant::now();
        self.processed_midi = None;
        self.active_processed_midi_id = None;
        self.active_display_cache_id = None;
        self.active_display_session_id = None;
        self.display.unload_midi(now);

        if self.active_audio_cache_id.is_none() {
            self.midi_cache = None;
            self.active_parsed_midi_id = None;
            self.midi_path = None;
            self.transport.reset(now);
            self.audio_clock.set_time(self.transport.current_time());
            self.audio_clock.set_playing(false);
        } else {
            self.display.mark_physics_tick(now);
        }

        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn unload_audio_context(&mut self) -> Vec<CoreEvent> {
        let now = Instant::now();
        self.audio_session = None;
        self.current_audio_cache = None;
        self.active_audio_cache_id = None;
        self.active_audio_session_id = None;
        self.audio_clock.set_playing(false);
        self.audio_player.reset();

        if self.active_display_cache_id.is_none() {
            self.midi_cache = None;
            self.active_parsed_midi_id = None;
            self.midi_path = None;
            self.transport.reset(now);
            self.audio_clock.set_time(self.transport.current_time());
        }

        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn unload_render_context(&mut self) -> Vec<CoreEvent> {
        let now = Instant::now();
        self.clear_active_midi_context(now);
        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn load_display_midi(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.0),
            "Opening MIDI file for display…",
        );
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }
        let parsed_midi_id =
            match self.ensure_parsed_midi_for_path(&path, 0.12, "Reading MIDI file…") {
                Ok(parsed_midi_id) => parsed_midi_id,
                Err(events) => return events,
            };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        let display_cache_id =
            match self.ensure_display_cache_for_path(parsed_midi_id, &path, 0.12, 0.94) {
                Ok(display_cache_id) => display_cache_id,
                Err(events) => return events,
            };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.94),
            "Attaching display cache…",
        );
        let attach_events = self.attach_display_cache_resource(display_cache_id);
        if !matches!(
            attach_events.as_slice(),
            [CoreEvent::DisplayCacheAttached { .. }]
        ) {
            return attach_events;
        }

        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.995),
            "Finalizing display load…",
        );
        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn load_audio_midi(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.0),
            "Opening MIDI file for audio…",
        );
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }
        let parsed_midi_id =
            match self.ensure_parsed_midi_for_path(&path, 0.12, "Reading MIDI file…") {
                Ok(parsed_midi_id) => parsed_midi_id,
                Err(events) => return events,
            };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        let audio_cache_id =
            match self.ensure_audio_cache_for_path(parsed_midi_id, &path, 0.12, 0.94) {
                Ok(audio_cache_id) => audio_cache_id,
                Err(events) => return events,
            };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.94),
            "Attaching audio cache…",
        );
        let attach_events = self.attach_audio_cache_resource(audio_cache_id);
        if !matches!(
            attach_events.as_slice(),
            [CoreEvent::AudioCacheAttached { .. }]
        ) {
            return attach_events;
        }

        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.995),
            "Finalizing audio load…",
        );
        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }

    pub(in crate::engine) fn load_midi_legacy(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.0),
            "Opening MIDI file…",
        );
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }
        let subscribers = Arc::clone(&self.subscribers);
        let parsed_progress_path = path.clone();
        let parsed_events =
            self.load_parsed_midi_resource_with_progress(path.clone(), move |phase| {
                super::support::broadcast_progress_to_subscribers(
                    &subscribers,
                    &parsed_progress_path,
                    MidiBuildProgress::from_fraction((phase * 0.1).clamp(0.0, 0.1)),
                    "Reading MIDI file…",
                );
            });
        let parsed_midi_id = match parsed_events.as_slice() {
            [event @ CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. }] => {
                self.broadcast(event.clone());
                self.broadcast_midi_load_progress(
                    &path,
                    MidiBuildProgress::default(),
                    "Building display and audio caches…",
                );
                parsed_midi_id
            }
            _ => return parsed_events,
        };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        let (display_cache_id, audio_cache_id) =
            match self.ensure_render_caches_for_path(*parsed_midi_id, &path, 0.1, 0.94) {
                Ok(ids) => ids,
                Err(events) => return events,
            };
        if let Err(events) = self.ensure_midi_load_not_cancelled(load_generation) {
            return events;
        }

        let display_attach_events = self.attach_display_cache_resource(display_cache_id);
        if !matches!(
            display_attach_events.as_slice(),
            [CoreEvent::DisplayCacheAttached { .. }]
        ) {
            return display_attach_events;
        }
        self.broadcast(display_attach_events[0].clone());
        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.97),
            "Attaching audio cache…",
        );

        let audio_attach_events = self.attach_audio_cache_resource(audio_cache_id);
        if !matches!(
            audio_attach_events.as_slice(),
            [CoreEvent::AudioCacheAttached { .. }]
        ) {
            return audio_attach_events;
        }
        self.broadcast(audio_attach_events[0].clone());
        self.broadcast_midi_load_progress(
            &path,
            MidiBuildProgress::from_fraction(0.995),
            "Finalizing MIDI load…",
        );

        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }
}
