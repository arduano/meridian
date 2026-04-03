use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    midi::{
        MidiCacheStack, ProcessedMidi, audio_cache::InRamAudioCache,
        display_cache::DisplayMidiCache,
    },
    protocol::{
        AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId,
        ProcessedMidiId,
    },
};

pub(super) struct ParsedMidiResource {
    pub(super) path: PathBuf,
    pub(super) cache_stack: MidiCacheStack,
}

pub(super) struct DisplayCacheResource {
    pub(super) parsed_midi_id: ParsedMidiId,
    pub(super) cache: Arc<DisplayMidiCache>,
}

pub(super) struct ProcessedMidiResource {
    pub(super) parsed_midi_id: ParsedMidiId,
    pub(super) midi: Arc<ProcessedMidi>,
}

pub(super) struct AudioCacheResource {
    pub(super) parsed_midi_id: ParsedMidiId,
    pub(super) cache: Arc<InRamAudioCache>,
}

pub(super) struct DisplaySessionResource {
    pub(super) display_cache_id: DisplayCacheId,
}

pub(super) struct AudioSessionResource {
    pub(super) audio_cache_id: AudioCacheId,
}

pub(super) type ParsedMidiRegistry = HashMap<ParsedMidiId, ParsedMidiResource>;
pub(super) type ProcessedMidiRegistry = HashMap<ProcessedMidiId, ProcessedMidiResource>;
pub(super) type DisplayCacheRegistry = HashMap<DisplayCacheId, DisplayCacheResource>;
pub(super) type AudioCacheRegistry = HashMap<AudioCacheId, AudioCacheResource>;
pub(super) type DisplaySessionRegistry = HashMap<DisplaySessionId, DisplaySessionResource>;
pub(super) type AudioSessionRegistry = HashMap<AudioSessionId, AudioSessionResource>;
