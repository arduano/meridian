use std::{path::PathBuf, sync::Arc, time::Instant};

use crate::{
    midi::{
        MidiCacheStack, MidiProcessingConfig, ProcessedMidi,
        analysis::{analyze_midi, analyze_parsed_midi_with_progress},
    },
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
    fn broadcast_midi_load_progress(
        &self,
        path: &std::path::Path,
        progress: Option<f32>,
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

    fn clear_active_midi_context(&mut self, now: Instant) {
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
        self.active_video_render_job_id = None;
        self.active_audio_render_job_id = None;
        self.midi_path = None;
        self.transport.reset(now);
        self.audio_clock.set_time(0.0);
        self.audio_clock.set_playing(false);
        self.audio_player.reset();
        self.display.unload_midi(now);
    }

    fn clear_conflicting_active_midi(&mut self, parsed_midi_id: ParsedMidiId) {
        if self.active_midi_conflicts_with(parsed_midi_id) {
            self.clear_active_midi_context(Instant::now());
        }
    }

    pub(super) fn load_parsed_midi_resource(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        self.load_parsed_midi_resource_with_progress(path, |_| {})
    }

    pub(super) fn load_parsed_midi_resource_with_progress(
        &mut self,
        path: PathBuf,
        progress: impl FnMut(f32),
    ) -> Vec<CoreEvent> {
        match MidiCacheStack::load_with_progress(&path, progress) {
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

    pub(super) fn build_display_cache_resource_with_progress(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        progress: impl FnMut(Option<f32>),
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        match parsed.cache_stack.display_cache_with_progress(progress) {
            Ok(cache_stack) => {
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
        self.build_audio_cache_resource_with_progress(parsed_midi_id, |_| {})
    }

    pub(super) fn build_audio_cache_resource_with_progress(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        progress: impl FnMut(Option<f32>),
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        match parsed.cache_stack.audio_cache_with_progress(progress) {
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
        let Some(parsed_midi_id) = self
            .display_caches
            .get(&display_cache_id)
            .map(|resource| resource.parsed_midi_id)
        else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown display_cache_id {}", display_cache_id.0),
            )];
        };
        self.clear_conflicting_active_midi(parsed_midi_id);
        let display_cache = Arc::clone(&self.display_caches[&display_cache_id].cache);
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "display cache is missing its parsed MIDI parent",
            )];
        };
        let parsed_path = parsed.path.clone();
        let parsed_stack = parsed.cache_stack.clone();

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
        let Some(parsed_midi_id) = self
            .audio_caches
            .get(&audio_cache_id)
            .map(|resource| resource.parsed_midi_id)
        else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown audio_cache_id {}", audio_cache_id.0),
            )];
        };
        self.clear_conflicting_active_midi(parsed_midi_id);
        let audio_cache = Arc::clone(&self.audio_caches[&audio_cache_id].cache);
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
        self.midi_path = Some(parsed.path.clone());
        self.current_audio_cache = Some(audio_cache);
        self.restart_audio_session();
        vec![CoreEvent::AudioCacheAttached {
            audio_cache_id,
            state: self.snapshot(),
        }]
    }

    pub(super) fn unload_display_context(&mut self) -> Vec<CoreEvent> {
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
            self.audio_clock.set_time(0.0);
            self.audio_clock.set_playing(false);
        } else {
            self.display.mark_physics_tick(now);
        }

        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(super) fn unload_audio_context(&mut self) -> Vec<CoreEvent> {
        let now = Instant::now();
        self.audio_session = None;
        self.current_audio_cache = None;
        self.active_audio_cache_id = None;
        self.active_audio_session_id = None;
        self.active_audio_render_job_id = None;
        self.audio_clock.set_playing(false);
        self.audio_player.reset();

        if self.active_display_cache_id.is_none() {
            self.midi_cache = None;
            self.active_parsed_midi_id = None;
            self.midi_path = None;
            self.transport.reset(now);
            self.audio_clock.set_time(0.0);
        }

        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(super) fn unload_render_context(&mut self) -> Vec<CoreEvent> {
        let now = Instant::now();
        self.clear_active_midi_context(now);
        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }

    pub(super) fn load_display_midi(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        self.broadcast_midi_load_progress(&path, Some(0.0), "Opening MIDI file for display…");
        let parsed_midi_id =
            match self.ensure_parsed_midi_for_path(&path, 0.12, "Reading MIDI file…") {
                Ok(parsed_midi_id) => parsed_midi_id,
                Err(events) => return events,
            };

        let display_cache_id =
            match self.ensure_display_cache_for_path(parsed_midi_id, &path, 0.12, 0.94) {
                Ok(display_cache_id) => display_cache_id,
                Err(events) => return events,
            };

        self.broadcast_midi_load_progress(&path, Some(0.94), "Attaching display cache…");
        let attach_events = self.attach_display_cache_resource(display_cache_id);
        if !matches!(
            attach_events.as_slice(),
            [CoreEvent::DisplayCacheAttached { .. }]
        ) {
            return attach_events;
        }

        self.broadcast_midi_load_progress(&path, Some(0.995), "Finalizing display load…");
        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }

    pub(super) fn load_audio_midi(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        self.broadcast_midi_load_progress(&path, Some(0.0), "Opening MIDI file for audio…");
        let parsed_midi_id =
            match self.ensure_parsed_midi_for_path(&path, 0.12, "Reading MIDI file…") {
                Ok(parsed_midi_id) => parsed_midi_id,
                Err(events) => return events,
            };

        let audio_cache_id =
            match self.ensure_audio_cache_for_path(parsed_midi_id, &path, 0.12, 0.94) {
                Ok(audio_cache_id) => audio_cache_id,
                Err(events) => return events,
            };

        self.broadcast_midi_load_progress(&path, Some(0.94), "Attaching audio cache…");
        let attach_events = self.attach_audio_cache_resource(audio_cache_id);
        if !matches!(
            attach_events.as_slice(),
            [CoreEvent::AudioCacheAttached { .. }]
        ) {
            return attach_events;
        }

        self.broadcast_midi_load_progress(&path, Some(0.995), "Finalizing audio load…");
        vec![CoreEvent::MidiLoaded {
            path,
            state: self.snapshot(),
        }]
    }

    pub(super) fn load_midi_legacy(&mut self, path: PathBuf) -> Vec<CoreEvent> {
        self.broadcast_midi_load_progress(&path, Some(0.0), "Opening MIDI file…");
        let subscribers = Arc::clone(&self.subscribers);
        let parsed_progress_path = path.clone();
        let parsed_events =
            self.load_parsed_midi_resource_with_progress(path.clone(), move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &parsed_progress_path,
                    Some((phase * 0.1).clamp(0.0, 0.1)),
                    "Reading MIDI file…",
                );
            });
        let parsed_midi_id = match parsed_events.as_slice() {
            [event @ CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. }] => {
                self.broadcast(event.clone());
                self.broadcast_midi_load_progress(
                    &path,
                    None,
                    "Building display and audio caches…",
                );
                *parsed_midi_id
            }
            _ => return parsed_events,
        };

        let (display_cache_id, audio_cache_id) = match self.ensure_render_caches_for_path(
            parsed_midi_id,
            &path,
            0.1,
            0.94,
        ) {
            Ok(ids) => ids,
            Err(events) => return events,
        };

        let display_attach_events = self.attach_display_cache_resource(display_cache_id);
        if !matches!(
            display_attach_events.as_slice(),
            [CoreEvent::DisplayCacheAttached { .. }]
        ) {
            return display_attach_events;
        }
        self.broadcast(display_attach_events[0].clone());
        self.broadcast_midi_load_progress(&path, Some(0.97), "Attaching audio cache…");

        let audio_attach_events = self.attach_audio_cache_resource(audio_cache_id);
        if !matches!(
            audio_attach_events.as_slice(),
            [CoreEvent::AudioCacheAttached { .. }]
        ) {
            return audio_attach_events;
        }
        self.broadcast(audio_attach_events[0].clone());
        self.broadcast_midi_load_progress(&path, Some(0.995), "Finalizing MIDI load…");

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
        let Some(parsed_midi_id) = self
            .processed_midis
            .get(&processed_midi_id)
            .map(|resource| resource.parsed_midi_id)
        else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown processed_midi_id {}", processed_midi_id.0),
            )];
        };
        self.clear_conflicting_active_midi(parsed_midi_id);
        let midi = Arc::clone(&self.processed_midis[&processed_midi_id].midi);
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
            let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active processed MIDI is missing its parsed MIDI parent",
                )];
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: Some(processed_midi_id),
                display_cache_id: None,
                analysis: analyze_midi(
                    parsed.cache_stack.parsed(),
                    resource.midi.display_cache().as_ref(),
                    resource.midi.analysis_cache().as_ref(),
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
            let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active display cache is missing its parsed MIDI parent",
                )];
            };
            let Ok(analysis) = parsed.cache_stack.analysis_cache() else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "failed to build analysis cache",
                )];
            };
            let analysis = match analyze_parsed_midi_with_progress(
                parsed.cache_stack.parsed(),
                analysis.as_ref(),
                bucket_count,
                |_| {},
            ) {
                Ok(analysis) => analysis,
                Err(error) => {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: None,
                display_cache_id: Some(display_cache_id),
                analysis,
            }];
        }

        if let Some(parsed_midi_id) = self.active_parsed_midi_id {
            let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active parsed MIDI is missing from registry",
                )];
            };
            let Ok(analysis) = parsed.cache_stack.analysis_cache() else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "failed to build analysis cache",
                )];
            };
            let analysis = match analyze_parsed_midi_with_progress(
                parsed.cache_stack.parsed(),
                analysis.as_ref(),
                bucket_count,
                |_| {},
            ) {
                Ok(analysis) => analysis,
                Err(error) => {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: None,
                display_cache_id: None,
                analysis,
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
        let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "processed midi is missing its parsed MIDI parent",
            )];
        };
        vec![CoreEvent::MidiAnalysis {
            processed_midi_id: Some(processed_midi_id),
            display_cache_id: None,
            analysis: analyze_midi(
                parsed.cache_stack.parsed(),
                resource.midi.display_cache().as_ref(),
                resource.midi.analysis_cache().as_ref(),
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

    fn ensure_parsed_midi_for_path(
        &mut self,
        path: &std::path::Path,
        progress_end: f32,
        status: &str,
    ) -> Result<ParsedMidiId, Vec<CoreEvent>> {
        if let Some(parsed_midi_id) = self
            .parsed_midis
            .iter()
            .find_map(|(id, resource)| (resource.path == path).then_some(*id))
        {
            self.broadcast_midi_load_progress(path, Some(progress_end), status);
            return Ok(parsed_midi_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events =
            self.load_parsed_midi_resource_with_progress(path.to_path_buf(), move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &progress_path,
                    Some((phase * progress_end).clamp(0.0, progress_end)),
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

    fn ensure_display_cache_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &std::path::Path,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<DisplayCacheId, Vec<CoreEvent>> {
        if let Some(display_cache_id) = self
            .display_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id))
        {
            self.broadcast_midi_load_progress(path, Some(progress_end), "Building display cache…");
            return Ok(display_cache_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events =
            self.build_display_cache_resource_with_progress(parsed_midi_id, move |phase| {
                broadcast_progress_to_subscribers(
                    &subscribers,
                    &progress_path,
                    phase.map(|phase| {
                        (progress_start + phase * (progress_end - progress_start))
                            .clamp(progress_start, progress_end)
                    }),
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

    fn ensure_audio_cache_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &std::path::Path,
        progress_start: f32,
        progress_end: f32,
    ) -> Result<AudioCacheId, Vec<CoreEvent>> {
        if let Some(audio_cache_id) = self
            .audio_caches
            .iter()
            .find_map(|(id, resource)| (resource.parsed_midi_id == parsed_midi_id).then_some(*id))
        {
            self.broadcast_midi_load_progress(path, Some(progress_end), "Building audio cache…");
            return Ok(audio_cache_id);
        }

        let subscribers = Arc::clone(&self.subscribers);
        let progress_path = path.to_path_buf();
        let events = self.build_audio_cache_resource_with_progress(parsed_midi_id, move |phase| {
            broadcast_progress_to_subscribers(
                &subscribers,
                &progress_path,
                phase.map(|phase| {
                    (progress_start + phase * (progress_end - progress_start))
                        .clamp(progress_start, progress_end)
                }),
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

    fn ensure_render_caches_for_path(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        path: &std::path::Path,
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
                Some(progress_end),
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
        let cache_result = parsed.cache_stack.render_caches_with_progress(move |phase| {
            broadcast_progress_to_subscribers(
                &subscribers,
                &progress_path,
                phase.map(|phase| {
                    (progress_start + phase * (progress_end - progress_start))
                        .clamp(progress_start, progress_end)
                }),
                "Building display and audio caches…",
            );
        });

        let (display_cache, audio_cache) = match cache_result {
            Ok(caches) => caches,
            Err(error) => return Err(vec![error_event(CoreErrorCode::Internal, error.to_string())]),
        };

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

fn broadcast_progress_to_subscribers(
    subscribers: &Arc<std::sync::Mutex<Vec<flume::Sender<CoreEvent>>>>,
    path: &std::path::Path,
    progress: Option<f32>,
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
