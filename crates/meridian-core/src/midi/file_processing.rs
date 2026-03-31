use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    convert::Infallible,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use midi_toolkit::{
    events::{
        ChannelPressureEvent, ControlChangeEvent, Event, MIDIEvent, NoteOffEvent, NoteOnEvent,
        PitchWheelChangeEvent, PolyphonicKeyPressureEvent, ProgramChangeEvent, TempoEvent,
    },
    io::MIDIWriter,
    sequence::event::{Delta, merge_events_array},
};

use crate::{
    error::MeridianError,
    protocol::{MidiProcessEvent, MidiProcessJobId},
};

use super::{
    parsed::ParsedMidiFile,
    processing::{
        EventFilterConfig, MidiFileProcessingConfig, MidiFileSelection, MidiMergeMode,
        NoteProcessingConfig, PitchProcessingConfig, StructureProcessingConfig,
        TrimProcessingConfig, ZeroVelocityNoteOnMode,
    },
    tempo_map::TempoMap,
    tool_pipeline::apply_modifier_tools,
};

#[derive(Debug, Clone)]
pub struct MidiFileProcessSummary {
    pub output: PathBuf,
    pub input_count: usize,
    pub output_track_count: usize,
    pub output_ppq: u16,
    pub total_events: usize,
}

#[derive(Debug, Clone)]
struct AbsoluteEvent {
    tick: u64,
    order: u64,
    event: Event,
}

#[derive(Debug)]
struct TrimCaptureState {
    tempo: Option<TempoEvent>,
    program_changes: Vec<Option<ProgramChangeEvent>>,
    channel_pressures: Vec<Option<ChannelPressureEvent>>,
    pitch_bends: Vec<Option<PitchWheelChangeEvent>>,
    poly_pressures: Vec<HashMap<u8, PolyphonicKeyPressureEvent>>,
    control_changes: Vec<HashMap<u8, ControlChangeEvent>>,
    open_notes: HashMap<(u8, u8), VecDeque<NoteOnEvent>>,
}

impl TrimCaptureState {
    fn new() -> Self {
        Self {
            tempo: None,
            program_changes: vec![None; 16],
            channel_pressures: vec![None; 16],
            pitch_bends: vec![None; 16],
            poly_pressures: vec![HashMap::new(); 16],
            control_changes: vec![HashMap::new(); 16],
            open_notes: HashMap::new(),
        }
    }
}

enum MappedEvent {
    Keep(Event),
    GeneratedNoteOff(NoteOffEvent),
    Drop,
}

pub fn process_midi_files_to_file(
    selection: &MidiFileSelection,
    output: &Path,
    config: &MidiFileProcessingConfig,
) -> Result<MidiFileProcessSummary, MeridianError> {
    process_midi_files_to_file_inner(
        selection,
        output,
        config,
        None,
        &AtomicBool::new(false),
        |_| {},
    )
}

pub fn process_midi_files_job(
    selection: &MidiFileSelection,
    output: &Path,
    config: &MidiFileProcessingConfig,
    job_id: MidiProcessJobId,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(MidiProcessEvent),
) -> Result<(), MeridianError> {
    on_event(MidiProcessEvent::ProcessStarted {
        job_id,
        output: output.to_path_buf(),
        total_inputs: selection.inputs.len(),
    });
    match process_midi_files_to_file_inner(
        selection,
        output,
        config,
        Some(job_id),
        cancel,
        &mut on_event,
    ) {
        Ok(summary) => {
            if cancel.load(Ordering::SeqCst) {
                on_event(MidiProcessEvent::ProcessCancelled {
                    job_id,
                    output: output.to_path_buf(),
                    processed_inputs: summary.input_count,
                    total_inputs: selection.inputs.len(),
                });
            } else {
                on_event(MidiProcessEvent::ProcessFinished {
                    job_id,
                    output: summary.output,
                    input_count: summary.input_count,
                    output_track_count: summary.output_track_count,
                    output_ppq: summary.output_ppq,
                    total_events: summary.total_events,
                });
            }
            Ok(())
        }
        Err(error) => {
            if cancel.load(Ordering::SeqCst) {
                on_event(MidiProcessEvent::ProcessCancelled {
                    job_id,
                    output: output.to_path_buf(),
                    processed_inputs: 0,
                    total_inputs: selection.inputs.len(),
                });
                Ok(())
            } else {
                on_event(MidiProcessEvent::ProcessFailed {
                    job_id,
                    output: output.to_path_buf(),
                    message: error.to_string(),
                });
                Err(error)
            }
        }
    }
}

fn process_midi_files_to_file_inner(
    selection: &MidiFileSelection,
    output: &Path,
    config: &MidiFileProcessingConfig,
    job_id: Option<MidiProcessJobId>,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(MidiProcessEvent),
) -> Result<MidiFileProcessSummary, MeridianError> {
    if selection.inputs.is_empty() {
        return Err(MeridianError::InvalidMidi(
            "midi file selection is empty".into(),
        ));
    }

    let mut parsed_inputs = Vec::with_capacity(selection.inputs.len());
    let mut output_ppq = config.time.ppq_override.unwrap_or(0);
    for path in &selection.inputs {
        if cancel.load(Ordering::SeqCst) {
            return Ok(MidiFileProcessSummary {
                output: output.to_path_buf(),
                input_count: 0,
                output_track_count: 0,
                output_ppq: output_ppq.max(1),
                total_events: 0,
            });
        }
        let parsed = ParsedMidiFile::load_from_file(path.clone())?;
        output_ppq = output_ppq.max(parsed.midi().ppq());
        parsed_inputs.push(parsed);
    }
    let output_ppq = output_ppq.max(1);

    let mut processed_files = Vec::new();
    let mut processed_input_count = 0usize;
    for (index, parsed) in parsed_inputs.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Ok(MidiFileProcessSummary {
                output: output.to_path_buf(),
                input_count: processed_input_count,
                output_track_count: 0,
                output_ppq,
                total_events: 0,
            });
        }
        let tempo_map = build_tempo_map(parsed)?;
        let file_tracks = process_single_midi(parsed, &tempo_map, output_ppq, config)?;
        processed_files.push(file_tracks);
        processed_input_count = index + 1;
        if let Some(job_id) = job_id {
            on_event(MidiProcessEvent::InputProgress {
                job_id,
                processed_inputs: processed_input_count,
                total_inputs: selection.inputs.len(),
                current_input: Some(selection.inputs[index].clone()),
            });
        }
    }

    let mut output_tracks = merge_processed_files(processed_files, config.merge.mode)?;
    if let Some(tempo) = config.time.tempo_override {
        inject_constant_tempo(&mut output_tracks, tempo);
    }
    apply_modifier_tools(&mut output_tracks, &config.tools)?;
    if config.structure.remove_empty_tracks {
        output_tracks.retain(|track| !track.is_empty());
    }

    let writer = MIDIWriter::new(output.to_string_lossy().as_ref(), output_ppq)
        .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
    let mut total_events = 0usize;
    for track in &output_tracks {
        if cancel.load(Ordering::SeqCst) {
            return Ok(MidiFileProcessSummary {
                output: output.to_path_buf(),
                input_count: processed_input_count,
                output_track_count: output_tracks.len(),
                output_ppq,
                total_events,
            });
        }
        let mut track_writer = writer.open_next_track();
        total_events += track_writer
            .write_events_iter(track.iter().cloned())
            .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
        track_writer
            .end()
            .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
    }
    let mut writer = writer;
    writer
        .end()
        .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;

    Ok(MidiFileProcessSummary {
        output: output.to_path_buf(),
        input_count: processed_input_count,
        output_track_count: output_tracks.len(),
        output_ppq,
        total_events,
    })
}

fn build_tempo_map(parsed: &ParsedMidiFile) -> Result<TempoMap, MeridianError> {
    let midi = parsed.midi();
    let mut map = TempoMap::new(midi.ppq());
    let mut tick = 0u64;
    for item in midi.iter_all_events_merged() {
        let event = item.map_err(|error| MeridianError::MidiLoad(format!("{error:?}")))?;
        tick = tick.saturating_add(event.delta);
        if let Event::Tempo(tempo) = event.event {
            let seconds = map.seconds_at_tick(tick);
            map.push_tempo_change(tick, seconds, tempo.tempo);
        }
    }
    Ok(map)
}

fn process_single_midi(
    parsed: &ParsedMidiFile,
    tempo_map: &TempoMap,
    output_ppq: u16,
    config: &MidiFileProcessingConfig,
) -> Result<Vec<Vec<Delta<u64, Event>>>, MeridianError> {
    let midi = parsed.midi();
    let trim = config.time.trim.as_ref();
    let trim_start = trim.map(|trim| trim.start_tick).unwrap_or(0);
    let trim_end = trim.and_then(|trim| trim.end_tick);
    let trim_start_seconds = if config.time.tempo_override.is_some() {
        tempo_map.seconds_at_tick(trim_start)
    } else {
        0.0
    };

    let mut processed_tracks = Vec::new();
    for (track_index, track_iter) in midi.iter_all_tracks().enumerate() {
        let events = track_iter
            .map(|event| event.map_err(|error| MeridianError::MidiLoad(format!("{error:?}"))))
            .collect::<Result<Vec<_>, _>>()?;
        let track_events = process_track_events(
            events,
            track_index as u32,
            midi.ppq(),
            output_ppq,
            tempo_map,
            trim,
            trim_start,
            trim_end,
            trim_start_seconds,
            config,
        );
        processed_tracks.extend(track_events);
    }

    if config.structure.collapse_tracks && processed_tracks.len() > 1 {
        return Ok(vec![merge_track_group(processed_tracks)?]);
    }

    if config.structure.remove_empty_tracks {
        processed_tracks.retain(|track| !track.is_empty());
    }
    Ok(processed_tracks)
}

#[allow(clippy::too_many_arguments)]
fn process_track_events(
    events: Vec<Delta<u64, Event>>,
    track_index: u32,
    source_ppq: u16,
    output_ppq: u16,
    tempo_map: &TempoMap,
    trim: Option<&TrimProcessingConfig>,
    trim_start: u64,
    trim_end: Option<u64>,
    trim_start_seconds: f64,
    config: &MidiFileProcessingConfig,
) -> Vec<Vec<Delta<u64, Event>>> {
    let mut absolute_tick = 0u64;
    let mut trim_state = TrimCaptureState::new();
    let mut active_open_notes: HashMap<(u8, u8), VecDeque<NoteOnEvent>> = HashMap::new();
    let mut seeded_active_open_notes = false;
    let mut order = 1_000_000u64;
    let mut buckets: BTreeMap<usize, Vec<AbsoluteEvent>> = BTreeMap::new();

    for event in events {
        absolute_tick = absolute_tick.saturating_add(event.delta);
        if trim_end.is_some_and(|end| absolute_tick >= end) {
            break;
        }

        let mapped = map_event(
            &event.event,
            &config.notes,
            &config.pitch,
            &config.events,
            config.piano_only,
            config.zero_velocity_note_on,
            config.time.tempo_override,
        );

        if absolute_tick < trim_start {
            update_trim_state(
                &mut trim_state,
                mapped,
                config.structure.drop_orphan_note_offs,
            );
            continue;
        }

        let Some(mapped_event) = event_to_event(mapped) else {
            continue;
        };
        if trim.is_some_and(|trim| trim.inject_edge_state) && !seeded_active_open_notes {
            active_open_notes = trim_state.open_notes.clone();
            seeded_active_open_notes = true;
        }
        let bucket = bucket_for_event(&mapped_event, &config.structure, track_index);
        buckets.entry(bucket).or_default().push(AbsoluteEvent {
            tick: absolute_tick,
            order,
            event: mapped_event,
        });
        let last = buckets.get(&bucket).and_then(|events| events.last());
        if let Some(last) = last {
            update_open_notes_from_event(
                &mut active_open_notes,
                &last.event,
                config.structure.drop_orphan_note_offs,
            );
        }
        order += 1;
    }

    if let Some(trim) = trim {
        if trim.inject_edge_state {
            inject_trim_state(
                &trim_state,
                &mut buckets,
                &config.structure,
                track_index,
                trim_start,
            );
            if !seeded_active_open_notes {
                active_open_notes = trim_state.open_notes.clone();
            }
        }
        if trim.close_open_notes_at_end {
            if let Some(trim_end) = trim_end {
                inject_trim_end_note_offs(
                    &active_open_notes,
                    &mut buckets,
                    &config.structure,
                    track_index,
                    trim_end,
                    order,
                );
            }
        }
    }

    buckets
        .into_values()
        .map(|bucket| {
            finalize_track_events(
                bucket,
                source_ppq,
                output_ppq,
                tempo_map,
                trim_start,
                trim_start_seconds,
                config,
            )
        })
        .filter(|track| !track.is_empty() || !config.structure.remove_empty_tracks)
        .collect()
}

fn update_trim_state(
    state: &mut TrimCaptureState,
    mapped: MappedEvent,
    drop_orphan_note_offs: bool,
) {
    match mapped {
        MappedEvent::Drop => {}
        MappedEvent::GeneratedNoteOff(note_off) => {
            let key = (note_off.channel, note_off.key);
            if let Some(queue) = state.open_notes.get_mut(&key) {
                queue.pop_front();
                if queue.is_empty() {
                    state.open_notes.remove(&key);
                }
            } else if !drop_orphan_note_offs {
                state
                    .open_notes
                    .entry(key)
                    .or_default()
                    .push_back(NoteOnEvent {
                        channel: note_off.channel,
                        key: note_off.key,
                        velocity: 1,
                    });
            }
        }
        MappedEvent::Keep(event) => match event {
            Event::NoteOn(note_on) => {
                state
                    .open_notes
                    .entry((note_on.channel, note_on.key))
                    .or_default()
                    .push_back(note_on);
            }
            Event::NoteOff(note_off) => {
                let key = (note_off.channel, note_off.key);
                if let Some(queue) = state.open_notes.get_mut(&key) {
                    queue.pop_front();
                    if queue.is_empty() {
                        state.open_notes.remove(&key);
                    }
                }
            }
            Event::Tempo(tempo) => state.tempo = Some(*tempo),
            Event::ProgramChange(program) => {
                state.program_changes[program.channel as usize] = Some(*program.clone());
            }
            Event::ControlChange(control) => {
                state.control_changes[control.channel as usize]
                    .insert(control.controller, *control);
            }
            Event::ChannelPressure(pressure) => {
                state.channel_pressures[pressure.channel as usize] = Some(*pressure.clone());
            }
            Event::PitchWheelChange(pitch) => {
                state.pitch_bends[pitch.channel as usize] = Some(*pitch.clone());
            }
            Event::PolyphonicKeyPressure(pressure) => {
                state.poly_pressures[pressure.channel as usize].insert(pressure.key, *pressure);
            }
            _ => {}
        },
    }
}

fn inject_trim_state(
    state: &TrimCaptureState,
    buckets: &mut BTreeMap<usize, Vec<AbsoluteEvent>>,
    structure: &StructureProcessingConfig,
    track_index: u32,
    trim_start: u64,
) {
    let mut order = 0u64;
    if let Some(tempo) = state.tempo.clone() {
        push_bucket_event(
            buckets,
            bucket_for_channel(None, structure, track_index),
            AbsoluteEvent {
                tick: trim_start,
                order,
                event: Event::Tempo(Box::new(tempo)),
            },
        );
        order += 1;
    }

    for channel in 0..16_u8 {
        if let Some(program) = state.program_changes[channel as usize].clone() {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::ProgramChange(Box::new(program)),
                },
            );
            order += 1;
        }
        for control in state.control_changes[channel as usize].values() {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::ControlChange(Box::new(control.clone())),
                },
            );
            order += 1;
        }
        if let Some(pressure) = state.channel_pressures[channel as usize].clone() {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::ChannelPressure(Box::new(pressure)),
                },
            );
            order += 1;
        }
        if let Some(pitch) = state.pitch_bends[channel as usize].clone() {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::PitchWheelChange(Box::new(pitch)),
                },
            );
            order += 1;
        }
        for pressure in state.poly_pressures[channel as usize].values() {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::PolyphonicKeyPressure(Box::new(pressure.clone())),
                },
            );
            order += 1;
        }
    }

    for note_ons in state.open_notes.values() {
        for note_on in note_ons {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(note_on.channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_start,
                    order,
                    event: Event::NoteOn(note_on.clone()),
                },
            );
            order += 1;
        }
    }
}

fn inject_trim_end_note_offs(
    open_notes: &HashMap<(u8, u8), VecDeque<NoteOnEvent>>,
    buckets: &mut BTreeMap<usize, Vec<AbsoluteEvent>>,
    structure: &StructureProcessingConfig,
    track_index: u32,
    trim_end: u64,
    mut order: u64,
) {
    for note_ons in open_notes.values() {
        for note_on in note_ons {
            push_bucket_event(
                buckets,
                bucket_for_channel(Some(note_on.channel), structure, track_index),
                AbsoluteEvent {
                    tick: trim_end,
                    order,
                    event: Event::NoteOff(NoteOffEvent {
                        channel: note_on.channel,
                        key: note_on.key,
                    }),
                },
            );
            order += 1;
        }
    }
}

fn finalize_track_events(
    mut bucket: Vec<AbsoluteEvent>,
    source_ppq: u16,
    output_ppq: u16,
    tempo_map: &TempoMap,
    trim_start: u64,
    trim_start_seconds: f64,
    config: &MidiFileProcessingConfig,
) -> Vec<Delta<u64, Event>> {
    if bucket.is_empty() {
        return Vec::new();
    }

    bucket.sort_by(|left, right| {
        left.tick
            .cmp(&right.tick)
            .then(left.order.cmp(&right.order))
    });
    let mut result = Vec::with_capacity(bucket.len());
    let mut previous_tick = 0u64;
    for event in bucket {
        let mapped_tick = map_output_tick(
            event.tick,
            source_ppq,
            output_ppq,
            tempo_map,
            trim_start,
            trim_start_seconds,
            config,
        );
        let delta = mapped_tick.saturating_sub(previous_tick);
        previous_tick = mapped_tick;
        result.push(Delta::new(delta, event.event));
    }
    result
}

fn map_output_tick(
    source_tick: u64,
    source_ppq: u16,
    output_ppq: u16,
    tempo_map: &TempoMap,
    trim_start: u64,
    trim_start_seconds: f64,
    config: &MidiFileProcessingConfig,
) -> u64 {
    let tick = if let Some(tempo_override) = config.time.tempo_override {
        let seconds = (tempo_map.seconds_at_tick(source_tick) - trim_start_seconds).max(0.0);
        let ticks_per_second =
            output_ppq.max(1) as f64 * 1_000_000.0 / tempo_override.max(1) as f64;
        (seconds * ticks_per_second).round() as u64
    } else {
        let relative = source_tick.saturating_sub(trim_start);
        scale_tick(relative, source_ppq, output_ppq)
    };

    apply_tick_offset(tick, config.time.offset_ticks)
}

fn scale_tick(tick: u64, from_ppq: u16, to_ppq: u16) -> u64 {
    if from_ppq == 0 {
        return tick;
    }
    ((tick as u128 * to_ppq as u128) / from_ppq as u128) as u64
}

fn apply_tick_offset(tick: u64, offset: i64) -> u64 {
    if offset >= 0 {
        tick.saturating_add(offset as u64)
    } else {
        tick.saturating_sub(offset.unsigned_abs())
    }
}

fn bucket_for_event(
    event: &Event,
    structure: &StructureProcessingConfig,
    track_index: u32,
) -> usize {
    bucket_for_channel(event.channel(), structure, track_index)
}

fn bucket_for_channel(
    channel: Option<u8>,
    structure: &StructureProcessingConfig,
    track_index: u32,
) -> usize {
    if structure.split_channels {
        channel.map(|channel| channel as usize + 1).unwrap_or(0)
    } else {
        track_index as usize
    }
}

fn push_bucket_event(
    buckets: &mut BTreeMap<usize, Vec<AbsoluteEvent>>,
    bucket: usize,
    event: AbsoluteEvent,
) {
    buckets.entry(bucket).or_default().push(event);
}

fn merge_processed_files(
    files: Vec<Vec<Vec<Delta<u64, Event>>>>,
    mode: MidiMergeMode,
) -> Result<Vec<Vec<Delta<u64, Event>>>, MeridianError> {
    match mode {
        MidiMergeMode::PreserveTracks => Ok(files.into_iter().flatten().collect()),
        MidiMergeMode::FlattenToSingleTrack => {
            let tracks: Vec<_> = files.into_iter().flatten().collect();
            if tracks.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![merge_track_group(tracks)?])
            }
        }
        MidiMergeMode::MergeByTrackIndex => {
            let mut grouped: BTreeMap<usize, Vec<Vec<Delta<u64, Event>>>> = BTreeMap::new();
            for file in files {
                for (index, track) in file.into_iter().enumerate() {
                    grouped.entry(index).or_default().push(track);
                }
            }
            grouped
                .into_values()
                .map(merge_track_group)
                .collect::<Result<Vec<_>, _>>()
        }
    }
}

fn update_open_notes_from_event(
    open_notes: &mut HashMap<(u8, u8), VecDeque<NoteOnEvent>>,
    event: &Event,
    drop_orphan_note_offs: bool,
) {
    match event {
        Event::NoteOn(note_on) => {
            open_notes
                .entry((note_on.channel, note_on.key))
                .or_default()
                .push_back(note_on.clone());
        }
        Event::NoteOff(note_off) => {
            let key = (note_off.channel, note_off.key);
            if let Some(queue) = open_notes.get_mut(&key) {
                queue.pop_front();
                if queue.is_empty() {
                    open_notes.remove(&key);
                }
            } else if !drop_orphan_note_offs {
                open_notes.entry(key).or_default();
            }
        }
        _ => {}
    }
}

fn merge_track_group(
    tracks: Vec<Vec<Delta<u64, Event>>>,
) -> Result<Vec<Delta<u64, Event>>, MeridianError> {
    let merged = merge_events_array(
        tracks
            .into_iter()
            .map(|track| track.into_iter().map(Ok::<_, Infallible>))
            .collect(),
    );
    let mut output = Vec::new();
    for event in merged {
        match event {
            Ok(event) => output.push(event),
            Err(error) => match error {},
        }
    }
    Ok(output)
}

fn inject_constant_tempo(tracks: &mut Vec<Vec<Delta<u64, Event>>>, tempo: u32) {
    if tracks.is_empty() {
        tracks.push(vec![Delta::new(
            0,
            Event::Tempo(Box::new(TempoEvent { tempo })),
        )]);
        return;
    }
    tracks[0].insert(
        0,
        Delta::new(0, Event::Tempo(Box::new(TempoEvent { tempo }))),
    );
}

fn event_to_event(mapped: MappedEvent) -> Option<Event> {
    match mapped {
        MappedEvent::Keep(event) => Some(event),
        MappedEvent::GeneratedNoteOff(note_off) => Some(Event::NoteOff(note_off)),
        MappedEvent::Drop => None,
    }
}

fn map_event(
    event: &Event,
    notes: &NoteProcessingConfig,
    pitch: &PitchProcessingConfig,
    filters: &EventFilterConfig,
    piano_only: bool,
    zero_velocity_mode: ZeroVelocityNoteOnMode,
    tempo_override: Option<u32>,
) -> MappedEvent {
    match event {
        Event::NoteOn(note_on) => {
            let velocity = notes.map_velocity(note_on.velocity);
            let Some(key) = notes.map_key(note_on.key) else {
                return MappedEvent::Drop;
            };
            if velocity == 0 && matches!(zero_velocity_mode, ZeroVelocityNoteOnMode::NoteOff) {
                return if filters.notes {
                    MappedEvent::GeneratedNoteOff(NoteOffEvent {
                        channel: note_on.channel,
                        key,
                    })
                } else {
                    MappedEvent::Drop
                };
            }
            if !filters.notes {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::NoteOn(NoteOnEvent {
                channel: note_on.channel,
                key,
                velocity,
            }))
        }
        Event::NoteOff(note_off) => {
            if !filters.notes {
                return MappedEvent::Drop;
            }
            let Some(key) = notes.map_key(note_off.key) else {
                return MappedEvent::Drop;
            };
            MappedEvent::Keep(Event::NoteOff(NoteOffEvent {
                channel: note_off.channel,
                key,
            }))
        }
        Event::PolyphonicKeyPressure(event) => {
            if !filters.polyphonic_pressure {
                return MappedEvent::Drop;
            }
            let Some(key) = notes.map_key(event.key) else {
                return MappedEvent::Drop;
            };
            MappedEvent::Keep(Event::PolyphonicKeyPressure(Box::new(
                PolyphonicKeyPressureEvent {
                    channel: event.channel,
                    key,
                    velocity: notes.map_velocity(event.velocity),
                },
            )))
        }
        Event::ControlChange(event) => {
            if !filters.channel_controls {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::ControlChange(Box::new(ControlChangeEvent {
                channel: event.channel,
                controller: event.controller,
                value: event.value,
            })))
        }
        Event::ProgramChange(event) => {
            if !filters.program_changes {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::ProgramChange(Box::new(ProgramChangeEvent {
                channel: event.channel,
                program: if piano_only { 0 } else { event.program },
            })))
        }
        Event::ChannelPressure(event) => {
            if !filters.channel_pressure {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::ChannelPressure(Box::new(ChannelPressureEvent {
                channel: event.channel,
                pressure: event.pressure,
            })))
        }
        Event::PitchWheelChange(event) => {
            if !filters.pitch_bend {
                return MappedEvent::Drop;
            }
            let bend =
                ((event.pitch as f32) * pitch.bend_scale).round() as i32 + pitch.bend_offset as i32;
            MappedEvent::Keep(Event::PitchWheelChange(Box::new(PitchWheelChangeEvent {
                channel: event.channel,
                pitch: bend.clamp(pitch.min_bend as i32, pitch.max_bend as i32) as i16,
            })))
        }
        Event::Tempo(event) => {
            if !filters.tempo || tempo_override.is_some() {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::Tempo(Box::new(TempoEvent { tempo: event.tempo })))
        }
        Event::SystemExclusiveMessage(event) => {
            if !filters.sysex {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::SystemExclusiveMessage(event.clone()))
        }
        Event::EndOfExclusive(event) => {
            if !filters.sysex {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::EndOfExclusive(event.clone()))
        }
        Event::Text(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::Text(event.clone()))
        }
        Event::UnknownMeta(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::UnknownMeta(event.clone()))
        }
        Event::ChannelPrefix(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::ChannelPrefix(event.clone()))
        }
        Event::MIDIPort(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::MIDIPort(event.clone()))
        }
        Event::SMPTEOffset(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::SMPTEOffset(event.clone()))
        }
        Event::TimeSignature(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::TimeSignature(event.clone()))
        }
        Event::KeySignature(event) => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(Event::KeySignature(event.clone()))
        }
        Event::Color(_) => MappedEvent::Drop,
        _ => {
            if !filters.meta_other {
                return MappedEvent::Drop;
            }
            MappedEvent::Keep(event.clone())
        }
    }
}
