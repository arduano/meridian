use std::{sync::Arc, time::Instant};

use crate::{
    engine::{error_code, support::error_event},
    midi::{MidiProcessingConfig, ProcessedMidi},
    protocol::{CoreErrorCode, CoreEvent, ParsedMidiId, ProcessedMidiId},
};

use super::super::{core_state::CoreState, resource_types::ProcessedMidiResource};

impl CoreState {
    pub(in crate::engine) fn build_processed_midi_resource(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        config: MidiProcessingConfig,
    ) -> Vec<CoreEvent> {
        let load_generation = self.core_handle.current_midi_load_generation();
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };
        let core_handle = self.core_handle.clone();
        match ProcessedMidi::from_parsed_cancelable(
            parsed.cache_stack.parsed(),
            &config,
            move || core_handle.current_midi_load_generation() != load_generation,
        ) {
            Ok(midi) => {
                if self.midi_load_was_cancelled(load_generation) {
                    return Self::midi_load_cancelled_events();
                }
                let processed_midi_id = self.resource_ids.next_processed_midi_id();
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
            Err(error) => vec![error_event(error_code(&error), error.to_string())],
        }
    }

    pub(in crate::engine) fn attach_processed_midi_resource(
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
        self.audio_clock.set_time(self.transport.current_time());
        self.audio_clock.set_playing(false);
        self.restart_audio_session();
        vec![CoreEvent::ProcessedMidiAttached {
            processed_midi_id,
            state: self.snapshot(),
        }]
    }
}
