use crate::protocol::{
    AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId, ProcessedMidiId,
};

use super::super::core_state::CoreState;

impl CoreState {
    pub(in crate::engine) fn next_parsed_midi_id(&mut self) -> ParsedMidiId {
        let id = ParsedMidiId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(in crate::engine) fn next_processed_midi_id(&mut self) -> ProcessedMidiId {
        let id = ProcessedMidiId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(in crate::engine) fn next_display_cache_id(&mut self) -> DisplayCacheId {
        let id = DisplayCacheId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(in crate::engine) fn next_audio_cache_id(&mut self) -> AudioCacheId {
        let id = AudioCacheId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(in crate::engine) fn next_display_session_id(&mut self) -> DisplaySessionId {
        let id = DisplaySessionId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }

    pub(in crate::engine) fn next_audio_session_id(&mut self) -> AudioSessionId {
        let id = AudioSessionId(self.next_resource_id);
        self.next_resource_id += 1;
        id
    }
}
