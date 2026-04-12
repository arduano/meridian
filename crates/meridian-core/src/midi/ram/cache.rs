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

    pub fn note_starts_between(&self, start_seconds: f64, end_seconds: f64) -> u64 {
        let start = start_seconds.min(end_seconds);
        let end = start_seconds.max(end_seconds);
        if end <= start {
            return 0;
        }
        self.columns
            .iter()
            .flat_map(|column| column.iter())
            .filter(|block| block.start_seconds >= start && block.start_seconds < end)
            .map(|block| block.note_count())
            .sum()
    }

    pub fn active_notes_at(&self, time_seconds: f64) -> u64 {
        self.columns
            .iter()
            .flat_map(|column| column.iter())
            .filter(|block| {
                block.start_seconds <= time_seconds
                    && block.start_seconds + block.max_length_seconds as f64 > time_seconds
            })
            .map(|block| block.active_notes_at_seconds(time_seconds))
            .sum()
    }
}
