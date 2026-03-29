use std::{collections::VecDeque, path::PathBuf};

use midi_toolkit::{
    events::{Event, MIDIEventEnum, TextEventKind},
    io::MIDIFile as TKMIDIFile,
    pipe,
    sequence::{
        TimeCaster, unwrap_items,
        event::{Delta, Track, cancel_tempo_events, scale_event_time},
    },
};
use rustc_hash::FxHashMap;

use crate::{
    error::MeridianError,
    midi::{
        MIDI_KEY_COUNT, MIDIColor, MIDIColorPair, TrackAndChannel, open_file_and_signature,
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
    block_builder: Vec<(TrackAndChannel, Option<MIDIColorPair>)>,
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

    fn add_note(&mut self, track_chan: TrackAndChannel, explicit_colors: Option<MIDIColorPair>) {
        let block_index = self.block_builder.len();
        let column_index = self.column.len();
        self.block_builder.push((track_chan, explicit_colors));
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
            self.column.push(InRamNoteBlock::new_from_notes(
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

impl InRamMIDIFile {
    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, MeridianError> {
        let (file, signature) = open_file_and_signature(path)?;
        let midi = TKMIDIFile::open_from_stream(file, None)
            .map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;
        let ppq = midi.ppq();
        if (ppq & 0x8000) != 0 {
            return Err(MeridianError::InvalidMidi(
                "timecode MIDI files are not supported yet".into(),
            ));
        }

        let merged = pipe!(
            midi.iter_all_track_events_merged()
            |>TimeCaster::<f64>::cast_event_delta()
            |>cancel_tempo_events(250000)
            |>scale_event_time(1.0 / ppq as f64)
            |>unwrap_items()
        );

        let mut keys: Vec<KeyBuilder> = (0..MIDI_KEY_COUNT).map(|_| KeyBuilder::new()).collect();
        let mut time = 0.0;
        let mut notes = 0_u64;
        let mut current_colors = vec![None; midi.track_count().max(1) * 16];

        type ToolkitEvent = Delta<f64, Track<Event>>;
        for event in merged {
            let event: ToolkitEvent = event;
            if event.delta > 0.0 {
                for key in &mut keys {
                    key.flush(time);
                }
                time += event.delta;
            }

            let track = event.track;
            match event.as_event() {
                Event::NoteOn(note_on) => {
                    let key_index = note_on.key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        let track_chan = TrackAndChannel::new(track, note_on.channel);
                        keys[key_index].add_note(track_chan, current_colors[track_chan.as_usize()]);
                        notes += 1;
                    }
                }
                Event::NoteOff(note_off) => {
                    let key_index = note_off.key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        let track_chan = TrackAndChannel::new(track, note_off.channel);
                        keys[key_index].end_note(track_chan, time);
                    }
                }
                Event::Text(text) if text.kind == TextEventKind::Undefined => {
                    if let Some((channel, pair)) = parse_color_event(&text.bytes) {
                        let track = track as usize;
                        match channel {
                            Some(channel) => {
                                let index = track * 16 + channel as usize;
                                if index < current_colors.len() {
                                    current_colors[index] = Some(pair);
                                }
                            }
                            None => {
                                let start = track * 16;
                                let end = (start + 16).min(current_colors.len());
                                current_colors[start..end].fill(Some(pair));
                            }
                        }
                    }
                }
                _ => {}
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
        let colors = MIDIColorPair::new_vec(midi.track_count().max(1));

        Ok(Self {
            view_data: InRamNoteViewData::new(columns, colors),
            length: time,
            note_count: notes,
            signature,
        })
    }
}

fn parse_color_event(data: &[u8]) -> Option<(Option<u8>, MIDIColorPair)> {
    if !(data.len() == 8 || data.len() == 12) || data[0] != 0x00 || data[1] != 0x0F {
        return None;
    }
    let channel = match data[2] {
        0x00..=0x0F => Some(data[2]),
        0x7F => None,
        _ => return None,
    };
    let left = MIDIColor::new(data[4], data[5], data[6]);
    let right = if data.len() == 12 {
        MIDIColor::new(data[8], data[9], data[10])
    } else {
        left
    };
    Some((channel, MIDIColorPair::new(left, right)))
}
