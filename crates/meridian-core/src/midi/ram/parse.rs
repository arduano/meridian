use std::{collections::VecDeque, sync::Arc};

use midi_toolkit::{
    events::{Event, MIDIEventEnum, TextEventKind},
    pipe,
    sequence::{
        event::{Delta, Track},
        unwrap_items,
    },
};
use rustc_hash::FxHashMap;

use crate::{
    error::MeridianError,
    midi::{
        MIDI_KEY_COUNT, MIDIColor, MIDIColorPair, TrackAndChannel,
        parsed::ParsedMidiFile,
        ram::{block::InRamNoteBlock, cache::InRamMidiCache},
        tempo_map::TempoMap,
    },
};

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

    fn end_note(&mut self, track_chan: TrackAndChannel, time_seconds: f64, time_ticks: u64) {
        let note = self
            .unended_notes
            .get_mut(&track_chan)
            .and_then(|queue| queue.pop_front());

        if let Some(note) = note {
            if note.column_index != self.column.len() {
                self.column[note.column_index].set_note_end_time(
                    note.block_index,
                    time_seconds,
                    time_ticks,
                );
            }
        }
    }

    fn flush(&mut self, time_seconds: f64, time_ticks: u64) {
        if !self.block_builder.is_empty() {
            self.column.push(InRamNoteBlock::new_from_notes(
                time_seconds,
                time_ticks,
                self.block_builder.drain(..),
            ));
        }
    }

    fn end_all(&mut self, time_seconds: f64, time_ticks: u64) {
        for (_, mut queue) in self.unended_notes.drain() {
            for note in queue.drain(..) {
                self.column[note.column_index].set_note_end_time(
                    note.block_index,
                    time_seconds,
                    time_ticks,
                );
            }
        }
    }
}

pub(super) fn build_in_ram_cache(parsed: &ParsedMidiFile) -> Result<InRamMidiCache, MeridianError> {
    build_in_ram_cache_with_progress(parsed, |_| {})
}

pub(super) fn build_in_ram_cache_with_progress(
    parsed: &ParsedMidiFile,
    mut progress: impl FnMut(f32),
) -> Result<InRamMidiCache, MeridianError> {
    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let merged = pipe!(midi.iter_all_track_events_merged() |> unwrap_items());

    let mut keys: Vec<KeyBuilder> = (0..MIDI_KEY_COUNT).map(|_| KeyBuilder::new()).collect();
    let mut time_seconds = 0.0;
    let mut time_ticks = 0_u64;
    let mut notes = 0_u64;
    let mut current_colors = vec![None; midi.track_count().max(1) * 16];
    let mut tempo_map = TempoMap::new(ppq);
    let mut micros_per_quarter = 500_000_u32;
    let total_events = parsed.total_event_count().max(1);
    let mut processed_events = 0_u64;
    let progress_stride = (total_events / 200).max(1);
    progress(0.0);

    type ToolkitEvent = Delta<u64, Track<Event>>;
    for event in merged {
        let event: ToolkitEvent = event;
        processed_events += 1;
        if event.delta > 0 {
            for key in &mut keys {
                key.flush(time_seconds, time_ticks);
            }
            time_ticks += event.delta;
            time_seconds +=
                event.delta as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq as f64;
        }

        let track = event.track;
        match event.as_event() {
            Event::Tempo(tempo) => {
                micros_per_quarter = tempo.tempo;
                tempo_map.push_tempo_change(time_ticks, time_seconds, micros_per_quarter);
            }
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
                    keys[key_index].end_note(track_chan, time_seconds, time_ticks);
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

        if processed_events == 1
            || processed_events >= total_events
            || processed_events % progress_stride == 0
        {
            progress((processed_events as f32 / total_events as f32).clamp(0.0, 1.0));
        }
    }

    for key in &mut keys {
        key.flush(time_seconds, time_ticks);
        key.end_all(time_seconds, time_ticks);
    }
    progress(1.0);

    let columns = keys
        .into_iter()
        .map(|key| Arc::<[InRamNoteBlock]>::from(key.column))
        .collect();

    Ok(InRamMidiCache::new(
        columns,
        time_seconds,
        notes,
        parsed.signature().clone(),
        midi.track_count().max(1),
        tempo_map,
    ))
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
