pub mod block;
pub mod column;
mod parse;
pub mod view;

use super::{MIDIAnalysisSummary, MIDIFile, MIDIFileBase, MIDIFileStats, MIDIFileUniqueSignature};
use view::{InRamCurrentNoteViews, InRamNoteViewData};

pub struct InRamMIDIFile {
    view_data: InRamNoteViewData,
    length: f64,
    note_count: u64,
    signature: MIDIFileUniqueSignature,
}

impl MIDIFileBase for InRamMIDIFile {
    fn midi_length(&self) -> Option<f64> {
        Some(self.length)
    }

    fn parsed_up_to(&self) -> Option<f64> {
        None
    }

    fn stats(&self) -> MIDIFileStats {
        MIDIFileStats {
            total_notes: Some(self.note_count),
            passed_notes: Some(self.view_data.passed_notes()),
        }
    }

    fn allows_seeking_backward(&self) -> bool {
        true
    }

    fn signature(&self) -> &MIDIFileUniqueSignature {
        &self.signature
    }
}

impl MIDIFile for InRamMIDIFile {
    type ColumnsViews<'a>
        = InRamCurrentNoteViews<'a>
    where
        Self: 'a;

    fn get_current_column_views(&mut self, time: f64, range: f64) -> Self::ColumnsViews<'_> {
        self.view_data
            .shift_view_range(super::MIDIViewRange::new(time, time + range));
        InRamCurrentNoteViews::new(&self.view_data)
    }
}

impl InRamMIDIFile {
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
        self.view_data.track_count()
    }
}
