use std::{path::PathBuf, sync::Arc, time::Instant};

use crate::{
    midi::{MidiCacheStack, MidiProcessingConfig, ProcessedMidi, analysis::analyze_display_cache},
    protocol::{
        AudioCacheId, AudioSessionId, CoreErrorCode, CoreEvent, DisplayCacheId, DisplaySessionId,
        ParsedMidiId, ProcessedMidiId,
    },
};

use super::{
    core_state::CoreState,
    resource_types::{
        AudioCacheResource, AudioSessionResource, DisplayCacheResource, DisplaySessionResource,
        ParsedMidiResource, ProcessedMidiResource,
    },
    support::error_event,
};

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
        self.processed_midi = None;
        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_processed_midi_id = None;
        self.active_display_cache_id = Some(display_cache_id);
        self.active_display_session_id = None;
        self.midi_path = Some(parsed_path);
        self.display.load_midi(
            crate::midi::MIDIFileUnion::InRam(display_cache.instantiate()),
            Instant::now(),
        );
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

        self.processed_midi = None;
        self.midi_cache = Some(parsed_stack);
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_processed_midi_id = None;
        self.active_audio_cache_id = Some(audio_cache_id);
        self.active_audio_session_id = None;
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
            [
                CoreEvent::DisplayCacheBuilt {
                    display_cache_id, ..
                },
            ] => *display_cache_id,
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

    pub(super) fn create_display_session_resource(
        &mut self,
        display_cache_id: DisplayCacheId,
    ) -> Vec<CoreEvent> {
        if !self.display_caches.contains_key(&display_cache_id) {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown display_cache_id {}", display_cache_id.0),
            )];
        }
        let display_session_id = self.next_display_session_id();
        self.display_sessions.insert(
            display_session_id,
            DisplaySessionResource { display_cache_id },
        );
        vec![CoreEvent::DisplaySessionCreated {
            display_session_id,
            display_cache_id,
        }]
    }

    pub(super) fn create_audio_session_resource(
        &mut self,
        audio_cache_id: AudioCacheId,
    ) -> Vec<CoreEvent> {
        if !self.audio_caches.contains_key(&audio_cache_id) {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown audio_cache_id {}", audio_cache_id.0),
            )];
        }
        let audio_session_id = self.next_audio_session_id();
        self.audio_sessions
            .insert(audio_session_id, AudioSessionResource { audio_cache_id });
        vec![CoreEvent::AudioSessionCreated {
            audio_session_id,
            audio_cache_id,
        }]
    }

    pub(super) fn attach_display_session_resource(
        &mut self,
        display_session_id: DisplaySessionId,
    ) -> Vec<CoreEvent> {
        let Some(session) = self.display_sessions.get(&display_session_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown display_session_id {}", display_session_id.0),
            )];
        };
        let display_cache_id = session.display_cache_id;
        let events = self.attach_display_cache_resource(display_cache_id);
        if matches!(events.as_slice(), [CoreEvent::DisplayCacheAttached { .. }]) {
            self.active_display_session_id = Some(display_session_id);
            return vec![CoreEvent::DisplaySessionAttached {
                display_session_id,
                state: self.snapshot(),
            }];
        }
        events
    }

    pub(super) fn attach_audio_session_resource(
        &mut self,
        audio_session_id: AudioSessionId,
    ) -> Vec<CoreEvent> {
        let Some(session) = self.audio_sessions.get(&audio_session_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown audio_session_id {}", audio_session_id.0),
            )];
        };
        let audio_cache_id = session.audio_cache_id;
        let events = self.attach_audio_cache_resource(audio_cache_id);
        if matches!(events.as_slice(), [CoreEvent::AudioCacheAttached { .. }]) {
            self.active_audio_session_id = Some(audio_session_id);
            return vec![CoreEvent::AudioSessionAttached {
                audio_session_id,
                state: self.snapshot(),
            }];
        }
        events
    }

    pub(super) fn next_parsed_midi_id(&mut self) -> ParsedMidiId {
        let id = ParsedMidiId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(super) fn build_processed_midi_resource(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        config: MidiProcessingConfig,
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        match ProcessedMidi::from_parsed(parsed.cache_stack.parsed(), &config) {
            Ok(midi) => {
                let processed_midi_id = self.next_processed_midi_id();
                let midi_length = midi.midi_length();
                let total_notes = midi.total_notes();
                let total_audio_events = midi.total_audio_events();
                let track_count = midi.track_count();
                self.processed_midis.insert(
                    processed_midi_id,
                    ProcessedMidiResource {
                        parsed_midi_id,
                        midi: Arc::new(midi),
                    },
                );
                vec![CoreEvent::ProcessedMidiBuilt {
                    parsed_midi_id,
                    processed_midi_id,
                    midi_length,
                    total_notes,
                    total_audio_events,
                    track_count,
                }]
            }
            Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
        }
    }

    pub(super) fn attach_processed_midi_resource(
        &mut self,
        processed_midi_id: ProcessedMidiId,
    ) -> Vec<CoreEvent> {
        let Some(resource) = self.processed_midis.get(&processed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown processed_midi_id {}", processed_midi_id.0),
            )];
        };
        let parsed_midi_id = resource.parsed_midi_id;
        let midi = Arc::clone(&resource.midi);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "processed midi is missing its parsed MIDI parent",
            )];
        };

        self.audio_session = None;
        self.processed_midi = Some(Arc::clone(&midi));
        self.midi_cache = Some(parsed.cache_stack.clone());
        self.active_parsed_midi_id = Some(parsed_midi_id);
        self.active_processed_midi_id = Some(processed_midi_id);
        self.active_display_cache_id = None;
        self.active_audio_cache_id = None;
        self.active_display_session_id = None;
        self.active_audio_session_id = None;
        self.midi_path = Some(parsed.path.clone());
        self.current_audio_cache = Some(midi.audio_cache());
        self.display.load_midi(
            crate::midi::MIDIFileUnion::InRam(midi.display_cache().instantiate()),
            Instant::now(),
        );
        if let Err(error) = self.refresh_note_colors() {
            return vec![error_event(CoreErrorCode::Internal, error.to_string())];
        }
        let now = Instant::now();
        self.transport.reset(now);
        self.display.mark_physics_tick(now);
        self.audio_clock.set_time(0.0);
        self.audio_clock.set_playing(false);
        self.restart_audio_session();
        vec![CoreEvent::ProcessedMidiAttached {
            processed_midi_id,
            state: self.snapshot(),
        }]
    }

    pub(super) fn next_processed_midi_id(&mut self) -> ProcessedMidiId {
        let id = ProcessedMidiId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(super) fn analyze_active_midi(&self, bucket_count: Option<usize>) -> Vec<CoreEvent> {
        let bucket_count = bucket_count.unwrap_or(1024);
        if let Some(processed_midi_id) = self.active_processed_midi_id {
            let Some(resource) = self.processed_midis.get(&processed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active processed MIDI is missing from registry",
                )];
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: Some(processed_midi_id),
                display_cache_id: None,
                analysis: analyze_display_cache(
                    resource.midi.display_cache().as_ref(),
                    bucket_count,
                ),
            }];
        }

        if let Some(display_cache_id) = self.active_display_cache_id {
            let Some(resource) = self.display_caches.get(&display_cache_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active display cache is missing from registry",
                )];
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: None,
                display_cache_id: Some(display_cache_id),
                analysis: analyze_display_cache(resource.cache.as_ref(), bucket_count),
            }];
        }

        vec![error_event(
            CoreErrorCode::NoMidiLoaded,
            "no active midi available for analysis",
        )]
    }

    pub(super) fn analyze_processed_midi(
        &self,
        processed_midi_id: ProcessedMidiId,
        bucket_count: Option<usize>,
    ) -> Vec<CoreEvent> {
        let Some(resource) = self.processed_midis.get(&processed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown processed_midi_id {}", processed_midi_id.0),
            )];
        };
        vec![CoreEvent::MidiAnalysis {
            processed_midi_id: Some(processed_midi_id),
            display_cache_id: None,
            analysis: analyze_display_cache(
                resource.midi.display_cache().as_ref(),
                bucket_count.unwrap_or(1024),
            ),
        }]
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

    pub(super) fn next_display_session_id(&mut self) -> DisplaySessionId {
        let id = DisplaySessionId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(super) fn next_audio_session_id(&mut self) -> AudioSessionId {
        let id = AudioSessionId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }
}
