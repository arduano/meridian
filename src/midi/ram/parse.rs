use std::{collections::VecDeque, io::Read, path::PathBuf};

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use rustc_hash::FxHashMap;

use crate::{
    error::MeridianError,
    midi::{
        MIDI_KEY_COUNT, MIDIColor, TrackAndChannel, open_file_and_signature,
        ram::{block::InRamNoteBlock, column::InRamNoteColumn, view::InRamNoteViewData},
    },
};

use super::InRamMIDIFile;

struct UnendedNote {
    column_index: usize,
    block_index: usize,
}

struct KeyBuilder {
    column: Vec<InRamNoteBlock>,
    block_builder: Vec<TrackAndChannel>,
    unended_notes: FxHashMap<TrackAndChannel, VecDeque<UnendedNote>>,
}

impl KeyBuilder {
    fn new() -> Self {
        Self {
            column: Vec::new(),
            block_builder: Vec::new(),
            unended_notes: FxHashMap::default(),
        }
    }

    fn add_note(&mut self, track_chan: TrackAndChannel) {
        let block_index = self.block_builder.len();
        let column_index = self.column.len();
        self.block_builder.push(track_chan);
        self.unended_notes
            .entry(track_chan)
            .or_default()
            .push_back(UnendedNote {
                column_index,
                block_index,
            });
    }

    fn end_note(&mut self, track_chan: TrackAndChannel, time: f64) {
        let note = self
            .unended_notes
            .get_mut(&track_chan)
            .and_then(|queue| queue.pop_front());

        if let Some(note) = note {
            if note.column_index != self.column.len() {
                self.column[note.column_index].set_note_end_time(note.block_index, time);
            }
        }
    }

    fn flush(&mut self, time: f64) {
        if !self.block_builder.is_empty() {
            self.column.push(InRamNoteBlock::new_from_trackchans(
                time,
                self.block_builder.drain(..),
            ));
        }
    }

    fn end_all(&mut self, time: f64) {
        for (_, mut queue) in self.unended_notes.drain() {
            for note in queue.drain(..) {
                self.column[note.column_index].set_note_end_time(note.block_index, time);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum EventKind {
    NoteOn { channel: u8, key: u8, velocity: u8 },
    NoteOff { channel: u8, key: u8 },
    Tempo { micros_per_quarter: u32 },
}

#[derive(Clone, Copy)]
struct EventRecord {
    tick: u64,
    track: u32,
    order: u64,
    kind: EventKind,
}

impl InRamMIDIFile {
    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, MeridianError> {
        let (mut file, signature) = open_file_and_signature(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let midi = Smf::parse(&bytes).map_err(|e| MeridianError::MidiLoad(e.to_string()))?;
        let ppq = match midi.header.timing {
            Timing::Metrical(value) => value.as_int() as u64,
            Timing::Timecode(_, _) => {
                return Err(MeridianError::InvalidMidi(
                    "timecode MIDI files are not supported yet".into(),
                ));
            }
        };

        let mut events = Vec::new();
        let mut order = 0_u64;

        for (track_index, track) in midi.tracks.iter().enumerate() {
            let mut absolute_tick = 0_u64;

            for event in track {
                absolute_tick += event.delta.as_int() as u64;

                match event.kind {
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::NoteOn { key, vel } => {
                            events.push(EventRecord {
                                tick: absolute_tick,
                                track: track_index as u32,
                                order,
                                kind: EventKind::NoteOn {
                                    channel: channel.as_int(),
                                    key: key.as_int(),
                                    velocity: vel.as_int(),
                                },
                            });
                            order += 1;
                        }
                        MidiMessage::NoteOff { key, .. } => {
                            events.push(EventRecord {
                                tick: absolute_tick,
                                track: track_index as u32,
                                order,
                                kind: EventKind::NoteOff {
                                    channel: channel.as_int(),
                                    key: key.as_int(),
                                },
                            });
                            order += 1;
                        }
                        _ => {}
                    },
                    TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                        events.push(EventRecord {
                            tick: absolute_tick,
                            track: track_index as u32,
                            order,
                            kind: EventKind::Tempo {
                                micros_per_quarter: tempo.as_int(),
                            },
                        });
                        order += 1;
                    }
                    _ => {}
                }
            }
        }

        events.sort_by_key(|event| (event.tick, event.order));

        let mut keys: Vec<KeyBuilder> = (0..MIDI_KEY_COUNT).map(|_| KeyBuilder::new()).collect();
        let mut time = 0.0;
        let mut last_tick = 0_u64;
        let mut current_tempo = 500_000_u32;
        let mut notes = 0_u64;

        for event in events {
            if event.tick > last_tick {
                for key in &mut keys {
                    key.flush(time);
                }

                let tick_delta = event.tick - last_tick;
                time += tick_delta as f64 * (current_tempo as f64 / ppq as f64) / 1_000_000.0;
                last_tick = event.tick;
            }

            match event.kind {
                EventKind::NoteOn {
                    channel,
                    key,
                    velocity,
                } if velocity > 0 => {
                    let key_index = key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        keys[key_index].add_note(TrackAndChannel::new(event.track, channel));
                        notes += 1;
                    }
                }
                EventKind::NoteOn { channel, key, .. } | EventKind::NoteOff { channel, key } => {
                    let key_index = key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        keys[key_index].end_note(TrackAndChannel::new(event.track, channel), time);
                    }
                }
                EventKind::Tempo { micros_per_quarter } => current_tempo = micros_per_quarter,
            }
        }

        for key in &mut keys {
            key.flush(time);
            key.end_all(time);
        }

        let columns = keys
            .into_iter()
            .map(|key| InRamNoteColumn::new(key.column))
            .collect();
        let colors = MIDIColor::new_vec(midi.tracks.len().max(1));

        Ok(Self {
            view_data: InRamNoteViewData::new(columns, colors),
            length: time,
            note_count: notes,
            signature,
        })
    }
}
