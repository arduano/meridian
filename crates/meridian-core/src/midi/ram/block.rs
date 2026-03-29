use crate::midi::{MIDIColorPair, TrackAndChannel};
use crate::render::DisplayTimeSpace;

pub struct InRamNoteBlock {
    pub start_seconds: f64,
    pub start_ticks: u64,
    pub max_length_seconds: f32,
    pub max_length_ticks: u32,
    pub(crate) notes: Box<[BasicMIDINote]>,
}

#[derive(Debug, Clone)]
pub(crate) struct BasicMIDINote {
    pub(crate) len_seconds: f32,
    pub(crate) len_ticks: u32,
    pub(crate) track_chan: TrackAndChannel,
    pub(crate) explicit_colors: Option<MIDIColorPair>,
}

impl InRamNoteBlock {
    /// Creates a new block from notes that all start at the same time.
    /// Lengths are filled in later when matching note-off events arrive.
    pub(crate) fn new_from_notes(
        start_seconds: f64,
        start_ticks: u64,
        notes: impl ExactSizeIterator<Item = (TrackAndChannel, Option<MIDIColorPair>)>,
    ) -> Self {
        let mut built = Vec::with_capacity(notes.len());
        for (track_chan, explicit_colors) in notes {
            built.push(BasicMIDINote {
                len_seconds: 0.0,
                len_ticks: 0,
                track_chan,
                explicit_colors,
            });
        }

        Self {
            start_seconds,
            start_ticks,
            max_length_seconds: 0.0,
            max_length_ticks: 0,
            notes: built.into_boxed_slice(),
        }
    }

    pub fn set_note_end_time(&mut self, note_index: usize, end_seconds: f64, end_ticks: u64) {
        let note = &mut self.notes[note_index];
        note.len_seconds = (end_seconds - self.start_seconds).max(0.0) as f32;
        note.len_ticks = end_ticks.saturating_sub(self.start_ticks) as u32;
        self.max_length_seconds = self.max_length_seconds.max(note.len_seconds);
        self.max_length_ticks = self.max_length_ticks.max(note.len_ticks);
    }

    pub fn start(&self, time_space: DisplayTimeSpace) -> f64 {
        match time_space {
            DisplayTimeSpace::Time => self.start_seconds,
            DisplayTimeSpace::Tick => self.start_ticks as f64,
        }
    }

    pub fn max_end(&self, time_space: DisplayTimeSpace) -> f64 {
        match time_space {
            DisplayTimeSpace::Time => self.start_seconds + self.max_length_seconds as f64,
            DisplayTimeSpace::Tick => self.start_ticks as f64 + self.max_length_ticks as f64,
        }
    }
}
