use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use crate::{
    midi::{MidiCacheStack, audio_cache::InRamAudioCache, display_cache::DisplayMidiCache},
    protocol::{AudioCacheId, CoreErrorCode, CoreEvent, DisplayCacheId, ParsedMidiId},
};

use super::{
    core_state::CoreState,
    support::error_event,
};

pub(super) struct ParsedMidiResource {
    pub(super) path: PathBuf,
    pub(super) cache_stack: MidiCacheStack,
}

pub(super) struct DisplayCacheResource {
    pub(super) parsed_midi_id: ParsedMidiId,
    pub(super) cache: Arc<DisplayMidiCache>,
}

pub(super) struct AudioCacheResource {
    pub(super) parsed_midi_id: ParsedMidiId,
    pub(super) cache: Arc<InRamAudioCache>,
}

pub(super) type ParsedMidiRegistry = HashMap<ParsedMidiId, ParsedMidiResource>;
pub(super) type DisplayCacheRegistry = HashMap<DisplayCacheId, DisplayCacheResource>;
pub(super) type AudioCacheRegistry = HashMap<AudioCacheId, AudioCacheResource>;

impl CoreState {
    pub(super) fn load_parsed_midi_resource(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        match MidiCacheStack::load(&path) {
            Ok(cache_stack) => {
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
            Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
        }
    }

    pub(super) fn build_display_cache_resource(
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
            Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
        }
    }

    pub(super) fn build_audio_cache_resource(
        &mut self,
        parsed_midi_id: ParsedMidiId,
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        match parsed.cache_stack.audio_cache() {
            Ok(cache) => {
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

    pub(super) fn attach_display_cache_resource(
        &mut self,
        display_cache_id: DisplayCacheId,
    ) -> Vec<CoreEvent> {
        let Some(resource) = self.display_caches.get(&display_cache_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown display_cache_id {}", display_cache_id.0),
            )];
        };
        let parsed_midi_id = resource.parsed_midi_id;
        let display_cache = Arc::clone(&resource.cache);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "display cache is missing its parsed MIDI parent",
            )];
        };
        let parsed_path = parsed.path.clone();
        let parsed_stack = parsed.cache_stack.clone();

        self.audio_session = None;
        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_display_cache_id = Some(display_cache_id);
        self.midi_path = Some(parsed_path);
        self.display
            .load_midi(crate::midi::MIDIFileUnion::InRam(display_cache.instantiate()), Instant::now());
        if let Err(error) = self.refresh_note_colors() {
            return vec![error_event(CoreErrorCode::Internal, error.to_string())];
        }
        let now = Instant::now();
        self.transport.reset(now);
        self.display.mark_physics_tick(now);
        self.audio_clock.set_time(0.0);
        self.audio_clock.set_playing(false);
        vec![CoreEvent::DisplayCacheAttached {
            display_cache_id,
            state: self.snapshot(),
        }]
    }

    pub(super) fn attach_audio_cache_resource(
        &mut self,
        audio_cache_id: AudioCacheId,
    ) -> Vec<CoreEvent> {
        let Some(resource) = self.audio_caches.get(&audio_cache_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown audio_cache_id {}", audio_cache_id.0),
            )];
        };
        let parsed_midi_id = resource.parsed_midi_id;
        let audio_cache = Arc::clone(&resource.cache);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "audio cache is missing its parsed MIDI parent",
            )];
        };
        let parsed_stack = parsed.cache_stack.clone();

        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_audio_cache_id = Some(audio_cache_id);
        self.current_audio_cache = Some(audio_cache);
        self.restart_audio_session();
        vec![CoreEvent::AudioCacheAttached {
            audio_cache_id,
            state: self.snapshot(),
        }]
    }

    pub(super) fn load_midi_legacy(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        let parsed_events = self.load_parsed_midi_resource(path.clone());
        let parsed_midi_id = match parsed_events.as_slice() {
            [CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. }] => *parsed_midi_id,
            _ => return parsed_events,
        };

        let display_events = self.build_display_cache_resource(parsed_midi_id);
        let display_cache_id = match display_events.as_slice() {
            [CoreEvent::DisplayCacheBuilt {
                display_cache_id, ..
            }] => *display_cache_id,
            _ => return display_events,
        };

        let audio_events = self.build_audio_cache_resource(parsed_midi_id);
        let audio_cache_id = match audio_events.as_slice() {
            [CoreEvent::AudioCacheBuilt { audio_cache_id, .. }] => *audio_cache_id,
            _ => return audio_events,
        };

        let display_attach_events = self.attach_display_cache_resource(display_cache_id);
        if !matches!(
            display_attach_events.as_slice(),
            [CoreEvent::DisplayCacheAttached { .. }]
        ) {
            return display_attach_events;
        }

        let audio_attach_events = self.attach_audio_cache_resource(audio_cache_id);
        if !matches!(
            audio_attach_events.as_slice(),
            [CoreEvent::AudioCacheAttached { .. }]
        ) {
            return audio_attach_events;
        }

        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }

    pub(super) fn next_parsed_midi_id(&mut self) -> ParsedMidiId {
        let id = ParsedMidiId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(super) fn next_display_cache_id(&mut self) -> DisplayCacheId {
        let id = DisplayCacheId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(super) fn next_audio_cache_id(&mut self) -> AudioCacheId {
        let id = AudioCacheId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }
}
