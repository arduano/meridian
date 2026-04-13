use std::sync::Arc;

use enum_dispatch::enum_dispatch;

use crate::{error::MeridianError, render::DisplayTimeSpace};

use super::{
    MIDI_KEY_COUNT, MIDIAnalysisSummary, MIDIColorPair, MIDIFile, MIDIFileBase, MIDIFileStats,
    MIDIFileUniqueSignature, MIDINoteColumnView, MIDINoteViews, MIDIViewRange,
    cache::MidiCacheStack, display_cache,
};

#[enum_dispatch(MIDIFileBase)]
/// Compatibility wrapper for the single concrete MIDI backing implementation.
///
/// The union exists so public APIs can stay stable if additional backing
/// storage types are introduced later, but today there is only one real
/// implementation.
pub enum MIDIFileUnion {
    InRam(super::ram::InRamMIDIFile),
}

impl MIDIFileUnion {
    pub fn load_ram(path: impl Into<std::path::PathBuf>) -> Result<Self, MeridianError> {
        MidiCacheStack::load(path)?.instantiate_display_in_ram()
    }

    pub fn get_current_column_views(
        &mut self,
        time: f64,
        range: f64,
        time_space: DisplayTimeSpace,
    ) -> MIDIFileViewsUnion<'_> {
        match self {
            Self::InRam(file) => {
                MIDIFileViewsUnion::InRam(file.get_current_column_views(time, range, time_space))
            }
        }
    }

    pub fn analysis_summary(&self) -> MIDIAnalysisSummary {
        match self {
            Self::InRam(file) => file.analysis_summary(),
        }
    }

    pub fn apply_default_track_colors(&mut self, colors: Vec<MIDIColorPair>) {
        match self {
            Self::InRam(file) => file.apply_default_track_colors(colors),
        }
    }

    pub fn track_count(&self) -> usize {
        match self {
            Self::InRam(file) => file.track_count(),
        }
    }

    pub fn key_note_counts(&self) -> [u64; MIDI_KEY_COUNT] {
        match self {
            Self::InRam(file) => file.key_note_counts(),
        }
    }

    pub fn display_cache(&self) -> Arc<display_cache::DisplayMidiCache> {
        match self {
            Self::InRam(file) => file.display_cache(),
        }
    }
}

pub enum MIDIFileViewsUnion<'a> {
    InRam(super::ram::view::InRamCurrentNoteViews<'a>),
}

impl MIDIFileViewsUnion<'_> {
    pub fn get_column(&self, key: usize) -> MIDINoteColumnViewUnion<'_> {
        match self {
            Self::InRam(views) => MIDINoteColumnViewUnion::InRam(views.get_column(key)),
        }
    }

    pub fn range(&self) -> MIDIViewRange {
        match self {
            Self::InRam(views) => views.range(),
        }
    }
}

pub enum MIDINoteColumnViewUnion<'a> {
    InRam(super::ram::view::InRamNoteColumnView<'a>),
}

impl MIDINoteColumnViewUnion<'_> {
    pub fn iterate_displaced_notes(
        &self,
    ) -> Box<dyn ExactSizeIterator<Item = super::DisplacedMIDINote> + Send + '_> {
        match self {
            Self::InRam(view) => Box::new(view.iterate_displaced_notes()),
        }
    }

    pub fn for_each_displaced_note(&self, mut callback: impl FnMut(super::DisplacedMIDINote)) {
        match self {
            Self::InRam(view) => {
                for note in view.iterate_displaced_notes() {
                    callback(note);
                }
            }
        }
    }
}
