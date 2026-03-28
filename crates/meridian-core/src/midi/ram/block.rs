use crate::midi::{MIDIColorPair, TrackAndChannel};

pub struct InRamNoteBlock {
    pub start: f64,
    pub max_length: f32,
    pub(crate) notes: Box<[BasicMIDINote]>,
}

#[derive(Debug, Clone)]
pub(crate) struct BasicMIDINote {
    pub(crate) len: f32,
    pub(crate) track_chan: TrackAndChannel,
    pub(crate) explicit_colors: Option<MIDIColorPair>,
}

impl InRamNoteBlock {
    pub(crate) fn new_from_notes(
        time: f64,
        notes: impl ExactSizeIterator<Item = (TrackAndChannel, Option<MIDIColorPair>)>,
    ) -> Self {
        let mut built = Vec::with_capacity(notes.len());
        for (track_chan, explicit_colors) in notes {
            built.push(BasicMIDINote {
                len: 0.0,
                track_chan,
                explicit_colors,
            });
        }

        Self {
            start: time,
            max_length: 0.0,
            notes: built.into_boxed_slice(),
        }
    }

    pub fn set_note_end_time(&mut self, note_index: usize, end_time: f64) {
        let note = &mut self.notes[note_index];
        note.len = (end_time - self.start).max(0.0) as f32;
        self.max_length = self.max_length.max(note.len);
    }

    pub fn max_end(&self) -> f64 {
        self.start + self.max_length as f64
    }
}
