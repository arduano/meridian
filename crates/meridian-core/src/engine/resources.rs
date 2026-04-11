mod caches;
mod gc;
mod ids;
mod loading;
mod processed;
mod sessions;
mod support;

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use flume::unbounded;

    use crate::audio::{AudioBackend, AudioConfig};
    use crate::engine::core_state::CoreState;
    use crate::midi::MidiProcessingConfig;
    use crate::protocol::CoreEvent;

    fn midi_fixture(relative_path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/midis")
            .join(relative_path)
    }

    fn test_state() -> CoreState {
        let (sender, _receiver) = unbounded();
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let mut state = CoreState::new(
            sender,
            subscribers,
            Arc::new(std::sync::atomic::AtomicU64::new(0)),
        );
        state.audio_config = AudioConfig {
            backend: AudioBackend::None,
            ..AudioConfig::default()
        };
        state
            .audio_player
            .switch(&state.audio_config)
            .expect("switch test audio backend");
        state
    }

    #[test]
    fn drop_inactive_midi_resources_reclaims_unloaded_preview_caches() {
        let mut state = test_state();

        let events =
            state.load_midi_legacy(midi_fixture("piano/burgmuller-op100-no13-consolation.mid"));
        assert!(matches!(events.as_slice(), [CoreEvent::MidiLoaded { .. }]));
        assert_eq!(state.parsed_midis.len(), 1);
        assert_eq!(state.display_caches.len(), 1);
        assert_eq!(state.audio_caches.len(), 1);

        state.unload_render_context();
        state.drop_inactive_midi_resources();

        assert!(state.parsed_midis.is_empty());
        assert!(state.processed_midis.is_empty());
        assert!(state.display_caches.is_empty());
        assert!(state.audio_caches.is_empty());
        assert!(state.display_sessions.is_empty());
        assert!(state.audio_sessions.is_empty());
    }

    #[test]
    fn drop_inactive_midi_resources_preserves_session_owned_caches() {
        let mut state = test_state();

        let events = state.load_display_midi(midi_fixture("smoke-two-notes.mid"));
        assert!(matches!(events.as_slice(), [CoreEvent::MidiLoaded { .. }]));
        let display_cache_id = state
            .active_display_cache_id
            .expect("display cache should be active");

        let session_events = state.create_display_session_resource(display_cache_id);
        assert!(matches!(
            session_events.as_slice(),
            [CoreEvent::DisplaySessionCreated { .. }]
        ));

        state.unload_display_context();
        state.drop_inactive_midi_resources();

        assert_eq!(state.parsed_midis.len(), 1);
        assert_eq!(state.display_caches.len(), 1);
        assert_eq!(state.display_sessions.len(), 1);
    }

    #[test]
    fn drop_inactive_midi_resources_reclaims_orphaned_analysis_artifacts() {
        let mut state = test_state();

        let parsed_events = state.load_parsed_midi_resource(midi_fixture("smoke-two-notes.mid"));
        let parsed_midi_id = match parsed_events.as_slice() {
            [CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. }] => *parsed_midi_id,
            other => panic!("unexpected parsed midi events: {other:?}"),
        };
        let processed_events =
            state.build_processed_midi_resource(parsed_midi_id, MidiProcessingConfig::default());
        assert!(matches!(
            processed_events.as_slice(),
            [CoreEvent::ProcessedMidiBuilt { .. }]
        ));
        assert_eq!(state.parsed_midis.len(), 1);
        assert_eq!(state.processed_midis.len(), 1);

        state.drop_inactive_midi_resources();

        assert!(state.parsed_midis.is_empty());
        assert!(state.processed_midis.is_empty());
    }
}
