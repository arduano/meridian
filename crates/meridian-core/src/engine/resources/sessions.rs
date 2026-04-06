use crate::protocol::{
    AudioCacheId, AudioSessionId, CoreErrorCode, CoreEvent, DisplayCacheId, DisplaySessionId,
};

use super::super::{
    core_state::CoreState,
    resource_types::{AudioSessionResource, DisplaySessionResource},
};

impl CoreState {
    pub(in crate::engine) fn create_display_session_resource(
        &mut self,
        display_cache_id: DisplayCacheId,
    ) -> Vec<CoreEvent> {
        if !self.display_caches.contains_key(&display_cache_id) {
            return vec![crate::engine::support::error_event(
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

    pub(in crate::engine) fn create_audio_session_resource(
        &mut self,
        audio_cache_id: AudioCacheId,
    ) -> Vec<CoreEvent> {
        if !self.audio_caches.contains_key(&audio_cache_id) {
            return vec![crate::engine::support::error_event(
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

    pub(in crate::engine) fn attach_display_session_resource(
        &mut self,
        display_session_id: DisplaySessionId,
    ) -> Vec<CoreEvent> {
        let Some(session) = self.display_sessions.get(&display_session_id) else {
            return vec![crate::engine::support::error_event(
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

    pub(in crate::engine) fn attach_audio_session_resource(
        &mut self,
        audio_session_id: AudioSessionId,
    ) -> Vec<CoreEvent> {
        let Some(session) = self.audio_sessions.get(&audio_session_id) else {
            return vec![crate::engine::support::error_event(
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
}
