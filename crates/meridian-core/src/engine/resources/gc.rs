use std::collections::HashSet;

use crate::protocol::{CoreEvent, MidiAnalysisJobStatus};

use super::super::core_state::CoreState;

impl CoreState {
    pub(in crate::engine) fn drop_inactive_midi_resources(&mut self) -> Vec<CoreEvent> {
        let mut keep_parsed = HashSet::new();
        let mut keep_processed = HashSet::new();
        let mut keep_display = HashSet::new();
        let mut keep_audio = HashSet::new();

        if let Some(parsed_midi_id) = self.active_parsed_midi_id {
            keep_parsed.insert(parsed_midi_id);
        }
        if let Some(processed_midi_id) = self.active_processed_midi_id {
            keep_processed.insert(processed_midi_id);
        }
        if let Some(display_cache_id) = self.active_display_cache_id {
            keep_display.insert(display_cache_id);
        }
        if let Some(audio_cache_id) = self.active_audio_cache_id {
            keep_audio.insert(audio_cache_id);
        }

        for session in self.display_sessions.values() {
            keep_display.insert(session.display_cache_id);
        }
        for session in self.audio_sessions.values() {
            keep_audio.insert(session.audio_cache_id);
        }

        for status in self.analysis_jobs.values() {
            if let MidiAnalysisJobStatus::Running { parsed_midi_id, .. } = status {
                keep_parsed.insert(*parsed_midi_id);
            }
        }

        for processed_midi_id in keep_processed.iter().copied().collect::<Vec<_>>() {
            if let Some(resource) = self.processed_midis.get(&processed_midi_id) {
                keep_parsed.insert(resource.parsed_midi_id);
            }
        }
        for display_cache_id in keep_display.iter().copied().collect::<Vec<_>>() {
            if let Some(resource) = self.display_caches.get(&display_cache_id) {
                keep_parsed.insert(resource.parsed_midi_id);
            }
        }
        for audio_cache_id in keep_audio.iter().copied().collect::<Vec<_>>() {
            if let Some(resource) = self.audio_caches.get(&audio_cache_id) {
                keep_parsed.insert(resource.parsed_midi_id);
            }
        }

        self.processed_midis
            .retain(|processed_midi_id, _| keep_processed.contains(processed_midi_id));
        self.display_caches
            .retain(|display_cache_id, _| keep_display.contains(display_cache_id));
        self.audio_caches
            .retain(|audio_cache_id, _| keep_audio.contains(audio_cache_id));
        self.display_sessions
            .retain(|_, session| keep_display.contains(&session.display_cache_id));
        self.audio_sessions
            .retain(|_, session| keep_audio.contains(&session.audio_cache_id));
        self.parsed_midis
            .retain(|parsed_midi_id, _| keep_parsed.contains(parsed_midi_id));

        vec![CoreEvent::StateSnapshot {
            state: self.snapshot(),
        }]
    }
}
