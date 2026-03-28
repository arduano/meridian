use crate::midi::TrackAndChannel;

pub struct InRamNoteBlock {
    pub start: f64,
    pub max_length: f32,
    pub(crate) notes: Box<[BasicMIDINote]>,
}

#[derive(Debug, Clone)]
pub(crate) struct BasicMIDINote {
    pub(crate) len: f32,
    pub(crate) track_chan: TrackAndChannel,
}

impl InRamNoteBlock {
    pub(crate) fn new_from_trackchans(
        time: f64,
        track_chans: impl ExactSizeIterator<Item = TrackAndChannel>,
    ) -> Self {
        let mut notes = Vec::with_capacity(track_chans.len());
        for track_chan in track_chans {
            notes.push(BasicMIDINote {
                len: 0.0,
                track_chan,
            });
        }

        Self {
            start: time,
            max_length: 0.0,
            notes: notes.into_boxed_slice(),
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
