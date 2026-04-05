use std::{collections::VecDeque, sync::Arc};

use midi_toolkit::{
    events::{Event, MIDIEventEnum},
    pipe,
    sequence::{
        event::{Delta, EventBatch, Track},
        unwrap_items,
    },
};
use rustc_hash::FxHashMap;

use crate::error::MeridianError;

use super::{
    MIDI_KEY_COUNT, TrackAndChannel,
    audio_cache::{CompressedAudio, InRamAudioCache},
    parsed::ParsedMidiFile,
    ram::{InRamMidiCache, block::InRamNoteBlock},
    tempo_map::TempoMap,
};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MaterializeOptions {
    pub display: bool,
    pub audio: bool,
}

pub(crate) struct MaterializedMidi {
    pub display: Option<InRamMidiCache>,
    pub audio: Option<InRamAudioCache>,
}

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

pub(crate) fn build_materialized_midi_with_progress(
    parsed: &ParsedMidiFile,
    options: MaterializeOptions,
    mut progress: impl FnMut(Option<f32>),
) -> Result<MaterializedMidi, MeridianError> {
    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let merged = pipe!(midi.iter_all_track_events_merged_batches() |> unwrap_items());
    let total_events = parsed.cached_total_event_count();
    let progress_stride = total_events.map(|events| (events.max(1) / 200).max(1));

    let mut keys = options
        .display
        .then(|| (0..MIDI_KEY_COUNT).map(|_| KeyBuilder::new()).collect::<Vec<_>>());
    let mut tempo_map = options.display.then(|| TempoMap::new(ppq));
    let mut dirty_keys = options.display.then(|| Vec::<usize>::new());
    let mut dirty_key_flags = options.display.then(|| [false; MIDI_KEY_COUNT]);
    let mut audio_blocks = options.audio.then(Vec::new);
    let mut current_audio_data = options.audio.then(Vec::new);
    let mut current_audio_control = options.audio.then(Vec::new);

    let mut time_seconds = 0.0;
    let mut time_ticks = 0_u64;
    let mut notes = 0_u64;
    let mut micros_per_quarter = 500_000_u32;
    let mut processed_events = 0_u64;

    progress(total_events.map(|_| 0.0));

    type ToolkitBatch = Delta<u64, Track<EventBatch<Event>>>;
    for batch in merged {
        let batch: ToolkitBatch = batch;
        let batch_event_count = batch.count() as u64;
        processed_events += batch_event_count;

        if batch.delta > 0 {
            if let Some(keys) = keys.as_mut() {
                if let (Some(dirty_keys), Some(dirty_key_flags)) =
                    (dirty_keys.as_mut(), dirty_key_flags.as_mut())
                {
                    for key_index in dirty_keys.drain(..) {
                        dirty_key_flags[key_index] = false;
                        keys[key_index].flush(time_seconds, time_ticks);
                    }
                }
            }
            if let (Some(audio_blocks), Some(current_audio_data), Some(current_audio_control)) = (
                audio_blocks.as_mut(),
                current_audio_data.as_mut(),
                current_audio_control.as_mut(),
            ) {
                flush_audio_block(
                    audio_blocks,
                    current_audio_data,
                    current_audio_control,
                    time_seconds,
                );
            }

            time_ticks += batch.delta;
            time_seconds +=
                batch.delta as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq as f64;
        }

        for event in batch.iter_events() {
            let track = event.track;

            match event.as_event() {
                Event::Tempo(tempo) => {
                    micros_per_quarter = tempo.tempo;
                    if let Some(tempo_map) = tempo_map.as_mut() {
                        tempo_map.push_tempo_change(time_ticks, time_seconds, micros_per_quarter);
                    }
                }
                Event::NoteOn(note_on) => {
                    let key_index = note_on.key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        let track_chan = TrackAndChannel::new(track, note_on.channel);
                        if note_on.velocity == 0 {
                            if let Some(keys) = keys.as_mut() {
                                keys[key_index].end_note(track_chan, time_seconds, time_ticks);
                            }
                        } else {
                            if let Some(keys) = keys.as_mut() {
                                let key = &mut keys[key_index];
                                let was_empty = key.block_builder.is_empty();
                                key.add_note(track_chan);
                                if was_empty {
                                    if let (Some(dirty_keys), Some(dirty_key_flags)) =
                                        (dirty_keys.as_mut(), dirty_key_flags.as_mut())
                                    {
                                        if !dirty_key_flags[key_index] {
                                            dirty_key_flags[key_index] = true;
                                            dirty_keys.push(key_index);
                                        }
                                    }
                                }
                            }
                            notes += 1;
                        }
                    }
                    if let (Some(current_audio_data), Some(current_audio_control)) = (
                        current_audio_data.as_mut(),
                        current_audio_control.as_mut(),
                    ) {
                        push_audio_event(event.as_event(), current_audio_data, current_audio_control);
                    }
                }
                Event::NoteOff(note_off) => {
                    let key_index = note_off.key as usize;
                    if key_index < MIDI_KEY_COUNT {
                        if let Some(keys) = keys.as_mut() {
                            let track_chan = TrackAndChannel::new(track, note_off.channel);
                            keys[key_index].end_note(track_chan, time_seconds, time_ticks);
                        }
                    }
                    if let (Some(current_audio_data), Some(current_audio_control)) = (
                        current_audio_data.as_mut(),
                        current_audio_control.as_mut(),
                    ) {
                        push_audio_event(event.as_event(), current_audio_data, current_audio_control);
                    }
                }
                _ => {
                    if let (Some(current_audio_data), Some(current_audio_control)) = (
                        current_audio_data.as_mut(),
                        current_audio_control.as_mut(),
                    ) {
                        push_audio_event(event.as_event(), current_audio_data, current_audio_control);
                    }
                }
            }
        }

        if let (Some(total_events), Some(progress_stride)) = (total_events, progress_stride) {
            if processed_events == batch_event_count
                || processed_events >= total_events
                || processed_events % progress_stride <= batch_event_count
            {
                progress(Some(
                    (processed_events as f32 / total_events as f32).clamp(0.0, 1.0),
                ));
            }
        }
    }

    let display = if let (Some(mut keys), Some(tempo_map)) = (keys, tempo_map) {
        if let (Some(dirty_keys), Some(dirty_key_flags)) = (dirty_keys.as_mut(), dirty_key_flags.as_mut()) {
            for key_index in dirty_keys.drain(..) {
                dirty_key_flags[key_index] = false;
                keys[key_index].flush(time_seconds, time_ticks);
            }
        }
        for key in keys.iter_mut() {
            key.end_all(time_seconds, time_ticks);
        }
        let columns = keys
            .into_iter()
            .map(|key| Arc::<[InRamNoteBlock]>::from(key.column))
            .collect();
        Some(InRamMidiCache::new(
            columns,
            time_seconds,
            notes,
            parsed.signature().clone(),
            midi.track_count().max(1),
            tempo_map,
        ))
    } else {
        None
    };

    let audio = if let (Some(mut audio_blocks), Some(mut current_audio_data), Some(mut current_audio_control)) =
        (audio_blocks, current_audio_data, current_audio_control)
    {
        flush_audio_block(
            &mut audio_blocks,
            &mut current_audio_data,
            &mut current_audio_control,
            time_seconds,
        );
        if !current_audio_data.is_empty() {
            audio_blocks.push(CompressedAudio::from_parts(
                time_seconds,
                current_audio_data,
                (!current_audio_control.is_empty()).then_some(current_audio_control),
            ));
        }
        Some(InRamAudioCache::new(audio_blocks))
    } else {
        None
    };

    progress(total_events.map(|_| 1.0));

    Ok(MaterializedMidi { display, audio })
}

fn flush_audio_block(
    audio_blocks: &mut Vec<CompressedAudio>,
    current_audio_data: &mut Vec<u8>,
    current_audio_control: &mut Vec<u8>,
    time_seconds: f64,
) {
    if !current_audio_data.is_empty() {
        audio_blocks.push(CompressedAudio::from_parts(
            time_seconds,
            std::mem::take(current_audio_data),
            (!current_audio_control.is_empty()).then(|| std::mem::take(current_audio_control)),
        ));
    }
}

fn push_audio_event(event: &Event, data: &mut Vec<u8>, control: &mut Vec<u8>) {
    const EV_OFF: u8 = 0x80;
    const EV_ON: u8 = 0x90;
    const EV_POLYPHONIC: u8 = 0xA0;
    const EV_CONTROL: u8 = 0xB0;
    const EV_PROGRAM: u8 = 0xC0;
    const EV_CHAN_PRESSURE: u8 = 0xD0;
    const EV_PITCH_BEND: u8 = 0xE0;

    match event {
        Event::NoteOn(e) => data.extend_from_slice(&[EV_ON | e.channel, e.key, e.velocity]),
        Event::NoteOff(e) => data.extend_from_slice(&[EV_OFF | e.channel, e.key]),
        Event::PolyphonicKeyPressure(e) => {
            data.extend_from_slice(&[EV_POLYPHONIC | e.channel, e.key, e.velocity]);
        }
        Event::ControlChange(e) => {
            let bytes = [EV_CONTROL | e.channel, e.controller, e.value];
            data.extend_from_slice(&bytes);
            control.extend_from_slice(&bytes);
        }
        Event::ProgramChange(e) => {
            let bytes = [EV_PROGRAM | e.channel, e.program];
            data.extend_from_slice(&bytes);
            control.extend_from_slice(&bytes);
        }
        Event::ChannelPressure(e) => {
            let bytes = [EV_CHAN_PRESSURE | e.channel, e.pressure];
            data.extend_from_slice(&bytes);
            control.extend_from_slice(&bytes);
        }
        Event::PitchWheelChange(e) => {
            let value = e.pitch + 8192;
            let bytes = [
                EV_PITCH_BEND | e.channel,
                (value & 0x7F) as u8,
                ((value >> 7) & 0x7F) as u8,
            ];
            data.extend_from_slice(&bytes);
            control.extend_from_slice(&bytes);
        }
        _ => {}
    }
}
