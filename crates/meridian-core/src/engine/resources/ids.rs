use crate::protocol::{
    AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId, ProcessedMidiId,
};

pub(in crate::engine) struct ResourceIds {
    next: u64,
}

impl ResourceIds {
    pub(in crate::engine) fn new(start: u64) -> Self {
        Self { next: start }
    }

    fn next_raw_id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    pub(in crate::engine) fn next_job_id(&mut self) -> u64 {
        self.next_raw_id()
    }

    pub(in crate::engine) fn next_parsed_midi_id(&mut self) -> ParsedMidiId {
        ParsedMidiId(self.next_raw_id())
    }

    pub(in crate::engine) fn next_processed_midi_id(&mut self) -> ProcessedMidiId {
        ProcessedMidiId(self.next_raw_id())
    }

    pub(in crate::engine) fn next_display_cache_id(&mut self) -> DisplayCacheId {
        DisplayCacheId(self.next_raw_id())
    }

    pub(in crate::engine) fn next_audio_cache_id(&mut self) -> AudioCacheId {
        AudioCacheId(self.next_raw_id())
    }

    pub(in crate::engine) fn next_display_session_id(&mut self) -> DisplaySessionId {
        DisplaySessionId(self.next_raw_id())
    }

    pub(in crate::engine) fn next_audio_session_id(&mut self) -> AudioSessionId {
        AudioSessionId(self.next_raw_id())
    }
}
