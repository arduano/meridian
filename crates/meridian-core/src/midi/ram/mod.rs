pub mod block;
mod cache;
pub mod column;
mod parse;
pub mod view;

pub use cache::InRamMidiCache;

use std::sync::Arc;

use crate::render::DisplayTimeSpace;

use super::{MIDIAnalysisSummary, MIDIFile, MIDIFileBase, MIDIFileStats, MIDIFileUniqueSignature};
use view::{InRamCurrentNoteViews, InRamNoteViewData};

pub struct InRamMIDIFile {
    cache: Arc<InRamMidiCache>,
    view_data: InRamNoteViewData,
}

impl MIDIFileBase for InRamMIDIFile {
    fn midi_length(&self) -> Option<f64> {
        Some(self.cache.length())
    }

    fn parsed_up_to(&self) -> Option<f64> {
        None
    }

    fn stats(&self) -> MIDIFileStats {
        MIDIFileStats {
            total_notes: Some(self.cache.note_count()),
            passed_notes: Some(self.view_data.passed_notes()),
        }
    }

    fn allows_seeking_backward(&self) -> bool {
        true
    }

    fn signature(&self) -> &MIDIFileUniqueSignature {
        self.cache.signature()
    }
}

impl MIDIFile for InRamMIDIFile {
    type ColumnsViews<'a>
        = InRamCurrentNoteViews<'a>
    where
        Self: 'a;

    fn get_current_column_views(
        &mut self,
        time: f64,
        range: f64,
        time_space: DisplayTimeSpace,
    ) -> Self::ColumnsViews<'_> {
        self.view_data
            .shift_view_range(self.cache.tempo_map(), time, range, time_space);
        InRamCurrentNoteViews::new(&self.view_data)
    }
}

impl InRamMIDIFile {
    pub(crate) fn from_cache(cache: Arc<InRamMidiCache>) -> Self {
        let view_data = InRamNoteViewData::from_cache(&cache);
        Self { cache, view_data }
    }

    pub fn analysis_summary(&self) -> MIDIAnalysisSummary {
        self.view_data.analysis_summary()
    }

    pub fn key_note_counts(&self) -> [u64; super::MIDI_KEY_COUNT] {
        self.view_data.key_note_counts()
    }

    pub fn apply_default_track_colors(&mut self, colors: Vec<super::MIDIColorPair>) {
        self.view_data.apply_default_track_colors(colors);
    }

    pub fn track_count(&self) -> usize {
        self.cache.track_count()
    }
}
