use std::{collections::VecDeque, sync::Arc};

use midi_toolkit::{
    events::Event,
    prelude::{EventSequenceExt, ResultIterExt},
};
use rustc_hash::FxHashMap;

use crate::{
    error::MeridianError,
    midi::{
        MIDIFileUniqueSignature, TrackAndChannel,
        analysis::{CachedMidiAnalysis, MidiAnalysisAccumulator},
        audio_cache::{CompressedAudio, InRamAudioCache},
        display_cache::DisplayMidiCache,
        parsed::ParsedMidiFile,
        processing::{MidiProcessingConfig, ZeroVelocityNoteOnMode},
        ram::block::InRamNoteBlock,
        tempo_map::TempoMap,
        traversal::walk_merged_midi_items,
    },
};

#[derive(Clone)]
pub struct ProcessedMidi {
    display: Arc<DisplayMidiCache>,
    analysis: Arc<CachedMidiAnalysis>,
    audio: Arc<InRamAudioCache>,
    signature: MIDIFileUniqueSignature,
    config: MidiProcessingConfig,
    midi_length: f64,
    total_notes: u64,
    total_audio_events: usize,
    track_count: usize,
}

impl ProcessedMidi {
    pub fn from_parsed(
        parsed: &ParsedMidiFile,
        config: &MidiProcessingConfig,
    ) -> Result<Self, MeridianError> {
        Self::from_parsed_cancelable(parsed, config, || false)
    }

    pub fn from_parsed_cancelable(
        parsed: &ParsedMidiFile,
        config: &MidiProcessingConfig,
        should_cancel: impl Fn() -> bool,
    ) -> Result<Self, MeridianError> {
        build_processed_midi_cancelable(parsed, config, should_cancel)
    }

    pub fn display_cache(&self) -> Arc<DisplayMidiCache> {
        Arc::clone(&self.display)
    }

    pub fn audio_cache(&self) -> Arc<InRamAudioCache> {
        Arc::clone(&self.audio)
    }

    pub fn analysis_cache(&self) -> Arc<CachedMidiAnalysis> {
        Arc::clone(&self.analysis)
    }

    pub fn signature(&self) -> &MIDIFileUniqueSignature {
        &self.signature
    }

    pub fn config(&self) -> &MidiProcessingConfig {
        &self.config
    }

    pub fn midi_length(&self) -> f64 {
        self.midi_length
    }

    pub fn total_notes(&self) -> u64 {
        self.total_notes
    }

    pub fn total_audio_events(&self) -> usize {
        self.total_audio_events
    }

    pub fn track_count(&self) -> usize {
        self.track_count
    }
}

#[derive(Clone)]
struct OpenNote {
    start: f64,
    track_chan: TrackAndChannel,
}

#[derive(Clone)]
struct FinishedNote {
    start: f64,
    end: f64,
    track_chan: TrackAndChannel,
}

fn build_processed_midi_cancelable(
    parsed: &ParsedMidiFile,
    config: &MidiProcessingConfig,
    should_cancel: impl Fn() -> bool,
) -> Result<ProcessedMidi, MeridianError> {
    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let merged = midi
        .iter_all_track_events_merged()
        .cast_event_delta::<f64>()
        .cancel_tempo_events(250000)
        .scale_event_time(1.0 / ppq as f64)
        .unwrap_items();

    let track_count = midi.track_count().max(1);
    let time = 0.0;
    let mut open_notes: FxHashMap<(u8, TrackAndChannel), VecDeque<OpenNote>> = FxHashMap::default();
    let mut finished_notes = vec![Vec::<FinishedNote>::new(); 256];
    let mut analysis = MidiAnalysisAccumulator::new(track_count);

    let mut current_audio_time = 0.0;
    let mut current_audio_data = Vec::new();
    let mut current_audio_control = Vec::new();
    let mut audio_blocks = Vec::new();
    struct ProcessedTraversalState<'a> {
        time: f64,
        current_output_time: f64,
        analysis: &'a mut MidiAnalysisAccumulator,
        open_notes: &'a mut FxHashMap<(u8, TrackAndChannel), VecDeque<OpenNote>>,
        finished_notes: &'a mut [Vec<FinishedNote>],
        current_audio_time: &'a mut f64,
        current_audio_data: &'a mut Vec<u8>,
        current_audio_control: &'a mut Vec<u8>,
        audio_blocks: &'a mut Vec<CompressedAudio>,
    }

    let mut traversal_state = ProcessedTraversalState {
        time,
        current_output_time: 0.0,
        analysis: &mut analysis,
        open_notes: &mut open_notes,
        finished_notes: &mut finished_notes,
        current_audio_time: &mut current_audio_time,
        current_audio_data: &mut current_audio_data,
        current_audio_control: &mut current_audio_control,
        audio_blocks: &mut audio_blocks,
    };

    walk_merged_midi_items(
        merged,
        &should_cancel,
        &mut traversal_state,
        |state, event| {
            state.time += event.delta;
            state.current_output_time = (state.time + config.time.offset_seconds).max(0.0);
            state
                .analysis
                .observe_time_advance(state.current_output_time);
            flush_audio_block(
                state.audio_blocks,
                state.current_audio_time,
                state.current_audio_data,
                state.current_audio_control,
                state.current_output_time,
            );
            Ok(())
        },
        |state, event| {
            let track = event.track();
            state
                .analysis
                .observe_event(event.as_event(), state.current_output_time);
            match event.as_event() {
                Event::NoteOn(note_on) => {
                    state.analysis.observe_note_on_velocity(note_on.velocity);
                    let velocity = config.notes.map_velocity(note_on.velocity);
                    let is_note_off = velocity == 0
                        && matches!(
                            config.zero_velocity_note_on,
                            ZeroVelocityNoteOnMode::NoteOff
                        );
                    if is_note_off {
                        end_note(
                            state.open_notes,
                            state.finished_notes,
                            state.analysis,
                            state.current_output_time,
                            note_on.key,
                            note_on.channel,
                            track,
                            config,
                        );
                        return Ok(());
                    }
                    if !config.events.notes {
                        return Ok(());
                    }
                    let Some(key) = config.notes.map_key(note_on.key) else {
                        return Ok(());
                    };
                    let track_chan = TrackAndChannel::new(track, note_on.channel);
                    state.analysis.observe_note_start(
                        state.current_output_time,
                        key as usize,
                        track_chan,
                    );
                    state
                        .open_notes
                        .entry((key, track_chan))
                        .or_default()
                        .push_back(OpenNote {
                            start: state.current_output_time,
                            track_chan,
                        });
                    state.current_audio_data.extend_from_slice(&[
                        0x90 | note_on.channel,
                        key,
                        velocity,
                    ]);
                }
                Event::NoteOff(note_off) => {
                    state.analysis.observe_note_off();
                    end_note(
                        state.open_notes,
                        state.finished_notes,
                        state.analysis,
                        state.current_output_time,
                        note_off.key,
                        note_off.channel,
                        track,
                        config,
                    );
                    if config.events.notes {
                        if let Some(key) = config.notes.map_key(note_off.key) {
                            state
                                .current_audio_data
                                .extend_from_slice(&[0x80 | note_off.channel, key]);
                        }
                    }
                }
                Event::PolyphonicKeyPressure(event) => {
                    if !config.events.polyphonic_pressure {
                        return Ok(());
                    }
                    if let Some(key) = config.notes.map_key(event.key) {
                        state.current_audio_data.extend_from_slice(&[
                            0xA0 | event.channel,
                            key,
                            config.notes.map_velocity(event.velocity),
                        ]);
                    }
                }
                Event::ControlChange(event) => {
                    if !config.events.channel_controls {
                        return Ok(());
                    }
                    let bytes = [0xB0 | event.channel, event.controller, event.value];
                    state.current_audio_data.extend_from_slice(&bytes);
                    state.current_audio_control.extend_from_slice(&bytes);
                }
                Event::ProgramChange(event) => {
                    if !config.events.program_changes {
                        return Ok(());
                    }
                    let program = if config.piano_only { 0 } else { event.program };
                    let bytes = [0xC0 | event.channel, program];
                    state.current_audio_data.extend_from_slice(&bytes);
                    state.current_audio_control.extend_from_slice(&bytes);
                }
                Event::ChannelPressure(event) => {
                    if !config.events.channel_pressure {
                        return Ok(());
                    }
                    let bytes = [0xD0 | event.channel, event.pressure];
                    state.current_audio_data.extend_from_slice(&bytes);
                    state.current_audio_control.extend_from_slice(&bytes);
                }
                Event::PitchWheelChange(event) => {
                    if !config.events.pitch_bend {
                        return Ok(());
                    }
                    let value = event.pitch + 8192;
                    let bytes = [
                        0xE0 | event.channel,
                        (value & 0x7F) as u8,
                        ((value >> 7) & 0x7F) as u8,
                    ];
                    state.current_audio_data.extend_from_slice(&bytes);
                    state.current_audio_control.extend_from_slice(&bytes);
                }
                _ => {}
            }
            Ok(())
        },
        |_, _| Ok(()),
    )?;
    drop(traversal_state);

    if should_cancel() {
        return Err(MeridianError::Cancelled("midi load cancelled".into()));
    }

    for ((key, _track_chan), queue) in &mut open_notes {
        while let Some(note) = queue.pop_front() {
            analysis.observe_note_release();
            finished_notes[*key as usize].push(FinishedNote {
                start: note.start,
                end: current_audio_time.max(note.start),
                track_chan: note.track_chan,
            });
        }
    }
    flush_final_audio_block(
        &mut audio_blocks,
        &mut current_audio_data,
        &mut current_audio_control,
        current_audio_time,
    );

    let mut note_count = 0_u64;
    let columns = finished_notes
        .into_iter()
        .enumerate()
        .map(|(key, mut key_notes)| {
            key_notes.sort_by(|a, b| a.start.total_cmp(&b.start));
            note_count += key_notes.len() as u64;
            Arc::<[InRamNoteBlock]>::from(notes_to_blocks(key, key_notes, &mut analysis))
        })
        .collect::<Vec<_>>();

    let midi_length = audio_blocks
        .last()
        .map(|event| event.time)
        .unwrap_or_else(|| columns_length(&columns));
    let analysis_cache = Arc::new(analysis.finalize(midi_length, note_count, track_count));
    let display = Arc::new(DisplayMidiCache::new(
        columns,
        midi_length,
        note_count,
        parsed.signature().clone(),
        track_count,
        TempoMap::new(parsed.midi().ppq()),
    ));
    let total_audio_events = audio_blocks.len();
    let audio = Arc::new(InRamAudioCache::new(audio_blocks));

    if should_cancel() {
        return Err(MeridianError::Cancelled("midi load cancelled".into()));
    }

    Ok(ProcessedMidi {
        display,
        analysis: analysis_cache,
        audio,
        signature: parsed.signature().clone(),
        config: config.clone(),
        midi_length,
        total_notes: note_count,
        total_audio_events,
        track_count,
    })
}

fn end_note(
    open_notes: &mut FxHashMap<(u8, TrackAndChannel), VecDeque<OpenNote>>,
    finished_notes: &mut [Vec<FinishedNote>],
    analysis: &mut MidiAnalysisAccumulator,
    output_time: f64,
    source_key: u8,
    channel: u8,
    track: u32,
    config: &MidiProcessingConfig,
) {
    let Some(key) = config.notes.map_key(source_key) else {
        return;
    };
    let track_chan = TrackAndChannel::new(track, channel);
    let Some(note) = open_notes
        .get_mut(&(key, track_chan))
        .and_then(|queue| queue.pop_front())
    else {
        return;
    };
    analysis.observe_note_release();
    if output_time >= note.start {
        // Duration stats are updated when the completed note is materialized.
        finished_notes[key as usize].push(FinishedNote {
            start: note.start,
            end: output_time,
            track_chan: note.track_chan,
        });
    }
}

fn notes_to_blocks(
    key: usize,
    notes: Vec<FinishedNote>,
    analysis: &mut MidiAnalysisAccumulator,
) -> Vec<InRamNoteBlock> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < notes.len() {
        let start = notes[index].start;
        let end = notes.partition_point(|note| note.start <= start);
        analysis.observe_block(key, end - index);
        let mut block = InRamNoteBlock::new_from_notes(
            start,
            0,
            notes[index..end].iter().map(|note| note.track_chan),
        );
        for (note_index, note) in notes[index..end].iter().enumerate() {
            block.set_note_end_time(note_index, note.end, 0);
            analysis.observe_note_end((note.end - note.start).max(0.0));
        }
        blocks.push(block);
        index = end;
    }
    blocks
}

fn flush_audio_block(
    audio_blocks: &mut Vec<CompressedAudio>,
    current_time: &mut f64,
    current_data: &mut Vec<u8>,
    current_control: &mut Vec<u8>,
    next_time: f64,
) {
    if next_time > *current_time && !current_data.is_empty() {
        audio_blocks.push(CompressedAudio::from_parts(
            *current_time,
            std::mem::take(current_data),
            (!current_control.is_empty()).then(|| std::mem::take(current_control)),
        ));
    }
    if next_time > *current_time {
        *current_time = next_time;
    }
}

fn flush_final_audio_block(
    audio_blocks: &mut Vec<CompressedAudio>,
    current_data: &mut Vec<u8>,
    current_control: &mut Vec<u8>,
    current_time: f64,
) {
    if !current_data.is_empty() {
        audio_blocks.push(CompressedAudio::from_parts(
            current_time,
            std::mem::take(current_data),
            (!current_control.is_empty()).then(|| std::mem::take(current_control)),
        ));
    }
}

fn columns_length(columns: &[Arc<[InRamNoteBlock]>]) -> f64 {
    columns
        .iter()
        .flat_map(|column| column.iter())
        .map(|block| block.max_end(crate::render::DisplayTimeSpace::Time))
        .fold(0.0, f64::max)
}
