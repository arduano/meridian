use std::{path::Path, sync::Arc};

use crate::{
    engine::{error_code, support::error_event},
    midi::{MidiBuildProgress, MidiCacheStack},
    protocol::{AudioCacheId, CoreErrorCode, CoreEvent, DisplayCacheId, ParsedMidiId},
};

use super::super::{
    core_state::CoreState,
    resource_types::{AudioCacheResource, DisplayCacheResource, ParsedMidiResource},
};
use super::support::{broadcast_progress_to_subscribers, scale_midi_build_progress};

impl CoreState {
    pub(in crate::engine) fn load_parsed_midi_resource(
        &mut self,
        path: std::path::PathBuf,
    ) -> Vec<CoreEvent> {
        self.load_parsed_midi_resource_with_progress(path, |_| {})
    }

    pub(in crate::engine) fn load_parsed_midi_resource_with_progress(
        &mut self,
        path: std::path::PathBuf,
        progress: impl FnMut(f32),
    ) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        let core_handle = self.core_handle.clone();
        match MidiCacheStack::load_with_progress_cancelable(&path, progress, move || {
            core_handle.current_midi_load_generation() != load_generation
        }) {
            Ok(cache_stack) => {
                if self.midi_load_was_cancelled(load_generation) {
                    return Self::midi_load_cancelled_events();
                }
                let parsed_midi_id = self.next_parsed_midi_id();
                self.parsed_midis.insert(
                    parsed_midi_id,
                    ParsedMidiResource {
                        path: path.clone(),
                        cache_stack,
                    },
                );
                vec![CoreEvent::ParsedMidiLoaded {
                    parsed_midi_id,
                    path,
                }]
            }
            Err(error) => vec![error_event(error_code(&error), error.to_string())],
        }
    }

    pub(in crate::engine) fn build_display_cache_resource_with_progress(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        progress: impl FnMut(MidiBuildProgress),
    ) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        let core_handle = self.core_handle.clone();
        match parsed
            .cache_stack
            .display_cache_with_progress_cancelable(progress, move || {
                core_handle.current_midi_load_generation() != load_generation
            }) {
            Ok(cache_stack) => {
                if self.midi_load_was_cancelled(load_generation) {
                    return Self::midi_load_cancelled_events();
                }
                let display_cache_id = self.next_display_cache_id();
                let event = CoreEvent::DisplayCacheBuilt {
                    parsed_midi_id,
                    display_cache_id,
                    midi_length: cache_stack.length(),
                    total_notes: cache_stack.note_count(),
                    track_count: cache_stack.track_count(),
                };
                self.display_caches.insert(
                    display_cache_id,
                    DisplayCacheResource {
                        parsed_midi_id,
                        cache: cache_stack,
                    },
                );
                vec![event]
            }
            Err(error) => vec![error_event(error_code(&error), error.to_string())],
        }
    }

    pub(in crate::engine) fn build_display_cache_resource(
        &mut self,
        parsed_midi_id: ParsedMidiId,
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        match parsed.cache_stack.display_cache() {
            Ok(cache) => {
                let display_cache_id = self.next_display_cache_id();
                let event = CoreEvent::DisplayCacheBuilt {
                    parsed_midi_id,
                    display_cache_id,
                    midi_length: cache.length(),
                    total_notes: cache.note_count(),
                    track_count: cache.track_count(),
                };
                self.display_caches.insert(
                    display_cache_id,
                    DisplayCacheResource {
                        parsed_midi_id,
                        cache,
                    },
                );
                vec![event]
            }
            Err(error) => vec![error_event(error_code(&error), error.to_string())],
        }
    }

    pub(in crate::engine) fn build_audio_cache_resource(
        &mut self,
        parsed_midi_id: ParsedMidiId,
    ) -> Vec<CoreEvent> {
        self.build_audio_cache_resource_with_progress(parsed_midi_id, |_| {})
    }

    pub(in crate::engine) fn build_audio_cache_resource_with_progress(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        progress: impl FnMut(MidiBuildProgress),
    ) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        let core_handle = self.core_handle.clone();
        match parsed
            .cache_stack
            .audio_cache_with_progress_cancelable(progress, move || {
                core_handle.current_midi_load_generation() != load_generation
            }) {
            Ok(cache) => {
                if self.midi_load_was_cancelled(load_generation) {
                    return Self::midi_load_cancelled_events();
                }
                let audio_cache_id = self.next_audio_cache_id();
                let total_events = cache.events().len();
                self.audio_caches.insert(
                    audio_cache_id,
                    AudioCacheResource {
                        parsed_midi_id,
                        cache,
                    },
                );
                vec![CoreEvent::AudioCacheBuilt {
                    parsed_midi_id,
                    audio_cache_id,
                    total_events,
                }]
            }
            Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
        }
    }

    pub(super) fn ensure_parsed_midi_for_path(
        &mut self,
        path: &Path,
        progress_end: f32,
        status: &str,
    ) -> Result<ParsedMidiId, Vec<CoreEvent>> {
        if let Some(parsed_midi_id) = self
            .parsed_midis
            .iter()
            .find_map(|(id, resource)| (resource.path == path).then_some(*id))
        {
            self.broadcast_midi_load_progress(
                path,
                MidiBuildProgress::from_fraction(progress_end),
                status,
            );
            return Ok(parsed_midi_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events =
            self.load_parsed_midi_resource_with_progress(path.to_path_buf(), move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &progress_path,
                    MidiBuildProgress::from_fraction(
                        (phase * progress_end).clamp(0.0, progress_end),
                    ),
                    status,
                );
            });
        match events.as_slice() {
            [event @ CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. }] => {
                self.broadcast(event.clone());
                Ok(*parsed_midi_id)
            }
            _ => Err(events),
        }
    }

    pub(super) fn ensure_display_cache_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &Path,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<DisplayCacheId, Vec<CoreEvent>> {
        if let Some(display_cache_id) = self
            .display_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id))
        {
            self.broadcast_midi_load_progress(
                path,
                MidiBuildProgress::from_fraction(progress_end),
                "Building display cache…",
            );
            return Ok(display_cache_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events =
            self.build_display_cache_resource_with_progress(parsed_midi_id, move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &progress_path,
                    scale_midi_build_progress(phase, progress_start, progress_end),
                    "Building display cache…",
                );
            });
        match events.as_slice() {
            [
                event @ CoreEvent::DisplayCacheBuilt {
                    display_cache_id, ..
                },
            ] => {
                self.broadcast(event.clone());
                Ok(*display_cache_id)
            }
            _ => Err(events),
        }
    }

    pub(super) fn ensure_audio_cache_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &Path,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<AudioCacheId, Vec<CoreEvent>> {
        if let Some(audio_cache_id) = self
            .audio_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id))
        {
            self.broadcast_midi_load_progress(
                path,
                MidiBuildProgress::from_fraction(progress_end),
                "Building audio cache…",
            );
            return Ok(audio_cache_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events = self.build_audio_cache_resource_with_progress(parsed_midi_id, move |phase| {
            broadcast_progress_to_subscribers(
                &subscribers,
                &progress_path,
                scale_midi_build_progress(phase, progress_start, progress_end),
                "Building audio cache…",
            );
        });
        match events.as_slice() {
            [event @ CoreEvent::AudioCacheBuilt { audio_cache_id, .. }] => {
                self.broadcast(event.clone());
                Ok(*audio_cache_id)
            }
            _ => Err(events),
        }
    }

    pub(super) fn ensure_render_caches_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &Path,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<(DisplayCacheId, AudioCacheId), Vec<CoreEvent>> {
        let existing_display = self
            .display_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id));
        let existing_audio = self
            .audio_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id));
        if let (Some(display_cache_id), Some(audio_cache_id)) = (existing_display, existing_audio) {
            self.broadcast_midi_load_progress(
                path,
                MidiBuildProgress::from_fraction(progress_end),
                "Building display and audio caches…",
            );
            return Ok((display_cache_id, audio_cache_id));
        }

        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return Err(vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )]);
        };

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let load_generation = self.core_handle.current_midi_load_generation();
        let core_handle = self.core_handle.clone();
        let cache_result = parsed.cache_stack.render_caches_with_progress_cancelable(
            move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &progress_path,
                    scale_midi_build_progress(phase, progress_start, progress_end),
                    "Building display and audio caches…",
                );
            },
            move || core_handle.current_midi_load_generation() != load_generation,
        );

        let (display_cache, audio_cache) = match cache_result {
            Ok(caches) => caches,
            Err(error) => {
                return Err(vec![error_event(error_code(&error), error.to_string())]);
            }
        };
        if self.midi_load_was_cancelled(load_generation) {
            return Err(Self::midi_load_cancelled_events());
        }

        let display_cache_id = if let Some(display_cache_id) = existing_display {
            display_cache_id
        } else {
            let display_cache_id = self.next_display_cache_id();
            self.display_caches.insert(
                display_cache_id,
                DisplayCacheResource {
                    parsed_midi_id,
                    cache: Arc::clone(&display_cache),
                },
            );
            self.broadcast(CoreEvent::DisplayCacheBuilt {
                parsed_midi_id,
                display_cache_id,
                midi_length: display_cache.length(),
                total_notes: display_cache.note_count(),
                track_count: display_cache.track_count(),
            });
            display_cache_id
        };

        let audio_cache_id = if let Some(audio_cache_id) = existing_audio {
            audio_cache_id
        } else {
            let audio_cache_id = self.next_audio_cache_id();
            self.audio_caches.insert(
                audio_cache_id,
                AudioCacheResource {
                    parsed_midi_id,
                    cache: Arc::clone(&audio_cache),
                },
            );
            self.broadcast(CoreEvent::AudioCacheBuilt {
                parsed_midi_id,
                audio_cache_id,
                total_events: audio_cache.events().len(),
            });
            audio_cache_id
        };

        Ok((display_cache_id, audio_cache_id))
    }
}
