use std::sync::Arc;

use crate::{
    error::MeridianError,
    midi::{MIDIColorPair, MIDIFileUniqueSignature, parsed::ParsedMidiFile, tempo_map::TempoMap},
};

use super::{InRamMIDIFile, block::InRamNoteBlock};

pub struct InRamMidiCache {
    columns: Vec<Arc<[InRamNoteBlock]>>,
    length: f64,
    note_count: u64,
    signature: MIDIFileUniqueSignature,
    track_count: usize,
    tempo_map: TempoMap,
}

impl InRamMidiCache {
    pub(crate) fn new(
        columns: Vec<Arc<[InRamNoteBlock]>>,
        length: f64,
        note_count: u64,
        signature: MIDIFileUniqueSignature,
        track_count: usize,
        tempo_map: TempoMap,
    ) -> Self {
        Self {
            columns,
            length,
            note_count,
            signature,
            track_count,
            tempo_map,
        }
    }

    pub fn from_parsed(parsed: &ParsedMidiFile) -> Result<Self, MeridianError> {
        super::parse::build_in_ram_cache(parsed)
    }

    pub fn from_parsed_with_progress(
        parsed: &ParsedMidiFile,
        progress: impl FnMut(crate::midi::MidiBuildProgress),
    ) -> Result<Self, MeridianError> {
        super::parse::build_in_ram_cache_with_progress(parsed, progress)
    }

    pub fn instantiate(self: &Arc<Self>) -> InRamMIDIFile {
        InRamMIDIFile::from_cache(Arc::clone(self))
    }

    pub(crate) fn columns(&self) -> &[Arc<[InRamNoteBlock]>] {
        &self.columns
    }

    pub(crate) fn default_colors(&self) -> Vec<MIDIColorPair> {
        MIDIColorPair::new_vec(self.track_count.max(1))
    }

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn note_count(&self) -> u64 {
        self.note_count
    }

    pub fn signature(&self) -> &MIDIFileUniqueSignature {
        &self.signature
    }

    pub fn track_count(&self) -> usize {
        self.track_count
    }

    pub fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }
}
