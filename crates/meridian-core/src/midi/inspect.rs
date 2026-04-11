use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use midi_toolkit::{
    events::{Event, MIDIEventEnum, TextEventKind},
    io::DiskReader,
    io::MIDIFile as RawToolkitMidiFile,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::MeridianError;

use super::{open_file_and_signature, parsed::ToolkitMidiFile};

/// Lightweight per-file inspection used by merge/source selection.
///
/// This contract intentionally excludes heavy analysis metrics such as gzip
/// size, note-duration statistics, polyphony, and buckets.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct MidiFileInspection {
    pub path: PathBuf,
    pub file_bytes: u64,
    pub midi_length: f64,
    pub total_notes: u64,
    pub total_event_count: u64,
    pub declared_track_count: u16,
    pub actual_track_count: usize,
    pub ticks_per_quarter: Option<u16>,
    pub tempo_event_count: u64,
    pub time_signature_event_count: u64,
    pub key_signature_event_count: u64,
    pub track_name_event_count: u64,
    pub initial_bpm: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct TempoPoint {
    tick: u64,
    micros_per_quarter: u32,
    track_index: u32,
    event_index: u32,
}

#[derive(Debug, Clone, Default)]
struct TrackInspectionPartial {
    total_event_count: u64,
    total_notes: u64,
    max_tick: u64,
    tempo_events: Vec<TempoPoint>,
    tempo_event_count: u64,
    time_signature_event_count: u64,
    key_signature_event_count: u64,
    track_name_event_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct ParsedInspectHeader {
    declared_track_count: u16,
    ticks_per_quarter: u16,
}

pub fn inspect_midi_files(paths: &[PathBuf]) -> Vec<MidiFileInspection> {
    paths.par_iter().cloned().map(inspect_midi_file).collect()
}

pub fn inspect_midi_file(path: PathBuf) -> MidiFileInspection {
    match inspect_midi_file_impl(path.clone()) {
        Ok(inspection) => inspection,
        Err(error) => MidiFileInspection {
            path,
            file_bytes: 0,
            midi_length: 0.0,
            total_notes: 0,
            total_event_count: 0,
            declared_track_count: 0,
            actual_track_count: 0,
            ticks_per_quarter: None,
            tempo_event_count: 0,
            time_signature_event_count: 0,
            key_signature_event_count: 0,
            track_name_event_count: 0,
            initial_bpm: 0.0,
            error: Some(error.to_string()),
        },
    }
}

fn inspect_midi_file_impl(path: PathBuf) -> Result<MidiFileInspection, MeridianError> {
    let (mut file, signature) = open_file_and_signature(path.clone())?;
    let header = read_header(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    let midi: ToolkitMidiFile = RawToolkitMidiFile::open_from_stream(file, None)
        .map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;

    let raw_track_count = midi.track_count();
    let actual_track_count = raw_track_count.max(1);
    let partials = (0..raw_track_count)
        .into_par_iter()
        .map(|track_index| inspect_track(&midi, track_index as u32))
        .collect::<Vec<_>>();

    let mut total_event_count = 0_u64;
    let mut total_notes = 0_u64;
    let mut max_tick = 0_u64;
    let mut tempo_events = Vec::new();
    let mut tempo_event_count = 0_u64;
    let mut time_signature_event_count = 0_u64;
    let mut key_signature_event_count = 0_u64;
    let mut track_name_event_count = 0_u64;

    for partial in partials {
        let partial = partial?;
        total_event_count += partial.total_event_count;
        total_notes += partial.total_notes;
        max_tick = max_tick.max(partial.max_tick);
        tempo_event_count += partial.tempo_event_count;
        time_signature_event_count += partial.time_signature_event_count;
        key_signature_event_count += partial.key_signature_event_count;
        track_name_event_count += partial.track_name_event_count;
        tempo_events.extend(partial.tempo_events);
    }

    tempo_events.sort_unstable_by_key(|event| (event.tick, event.track_index, event.event_index));
    let (midi_length, initial_bpm) =
        compute_tempo_timeline(header.ticks_per_quarter, max_tick, &tempo_events);

    Ok(MidiFileInspection {
        path,
        file_bytes: signature.length_in_bytes,
        midi_length,
        total_notes,
        total_event_count,
        declared_track_count: header.declared_track_count,
        actual_track_count,
        ticks_per_quarter: Some(header.ticks_per_quarter),
        tempo_event_count,
        time_signature_event_count,
        key_signature_event_count,
        track_name_event_count,
        initial_bpm,
        error: None,
    })
}

fn inspect_track(
    midi: &RawToolkitMidiFile<DiskReader>,
    track_index: u32,
) -> Result<TrackInspectionPartial, MeridianError> {
    let mut absolute_tick = 0_u64;
    let mut event_index = 0_u32;
    let mut partial = TrackInspectionPartial::default();
    let track = midi
        .iter_track(track_index)
        .ok_or_else(|| MeridianError::InvalidMidi(format!("missing track {}", track_index)))?;

    for event in track {
        let event = event.map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;
        absolute_tick = absolute_tick.saturating_add(event.delta);
        partial.max_tick = partial.max_tick.max(absolute_tick);
        partial.total_event_count += 1;

        match event.as_event() {
            Event::NoteOn(note_on) if note_on.velocity > 0 => {
                partial.total_notes += 1;
            }
            Event::Tempo(tempo) => {
                partial.tempo_event_count += 1;
                partial.tempo_events.push(TempoPoint {
                    tick: absolute_tick,
                    micros_per_quarter: tempo.tempo.max(1),
                    track_index,
                    event_index,
                });
            }
            Event::TimeSignature(_) => {
                partial.time_signature_event_count += 1;
            }
            Event::KeySignature(_) => {
                partial.key_signature_event_count += 1;
            }
            Event::Text(text) if text.kind == TextEventKind::TrackName => {
                partial.track_name_event_count += 1;
            }
            _ => {}
        }

        event_index = event_index.saturating_add(1);
    }

    Ok(partial)
}

fn compute_tempo_timeline(
    ticks_per_quarter: u16,
    max_tick: u64,
    tempo_events: &[TempoPoint],
) -> (f64, f64) {
    let ppq = ticks_per_quarter.max(1) as f64;
    let mut micros_per_quarter = 500_000_u32;
    let mut current_tick = 0_u64;
    let mut midi_length = 0.0;
    let mut initial_bpm = 120.0;

    for event in tempo_events {
        if event.tick > current_tick {
            midi_length +=
                (event.tick - current_tick) as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq;
            current_tick = event.tick;
        }
        micros_per_quarter = event.micros_per_quarter;
        if event.tick == 0 {
            initial_bpm = 60_000_000.0 / micros_per_quarter as f64;
        }
    }

    if max_tick > current_tick {
        midi_length +=
            (max_tick - current_tick) as f64 * micros_per_quarter as f64 / 1_000_000.0 / ppq;
    }

    (midi_length, initial_bpm)
}

fn read_header(file: &mut File) -> Result<ParsedInspectHeader, MeridianError> {
    let mut header = [0_u8; 14];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut header)?;
    if &header[0..4] != b"MThd" {
        return Err(MeridianError::InvalidMidi(
            "missing MIDI header chunk".into(),
        ));
    }

    let time_division = u16::from_be_bytes([header[12], header[13]]);
    if (time_division & 0x8000) != 0 {
        return Err(MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    Ok(ParsedInspectHeader {
        declared_track_count: u16::from_be_bytes([header[10], header[11]]),
        ticks_per_quarter: time_division,
    })
}
