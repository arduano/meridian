use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

use midi_toolkit::events::{Event, MIDIEventEnum, TextEventKind};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::midi::parsed::{ParsedMidiFile, ToolkitMidiFile};

use super::{AnalysisProgressUpdate, types::MidiAnalysisEventMetrics};

#[derive(Debug, Clone)]
pub(crate) struct TempoEventPoint {
    pub(crate) tick: u64,
    pub(crate) track: u32,
    pub(crate) event_index: u32,
    pub(crate) micros_per_quarter: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct TickTransitionPoint {
    pub(crate) tick: u64,
    pub(crate) track: u32,
    pub(crate) net_delta: i64,
    pub(crate) prefix_peak_delta: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct NoteIntervalAggregate {
    pub(crate) start_tick: u64,
    pub(crate) end_tick: u64,
    pub(crate) count: u64,
}

#[derive(Debug)]
struct TrackAnalysisPartial {
    track_index: u32,
    total_event_count: u64,
    total_notes: u64,
    max_tick: u64,
    event_metrics: MidiAnalysisEventMetrics,
    velocity_note_on_counts: [u64; 128],
    key_note_counts: [u64; 256],
    pitch_class_note_counts: [u64; 12],
    channel_note_counts: [u64; 16],
    track_channel_note_counts: [u64; 16],
    tempo_events: Vec<TempoEventPoint>,
    onset_counts: Vec<(u64, u64)>,
    block_starts: Vec<(u64, u8, u64)>,
    tick_transitions: Vec<TickTransitionPoint>,
    intervals: Vec<NoteIntervalAggregate>,
    unresolved_starts: Vec<(u64, u64)>,
}

#[derive(Debug)]
pub(crate) struct ExtractedAnalysis {
    pub(crate) ppq: u16,
    pub(crate) actual_track_count: usize,
    pub(crate) total_event_count: u64,
    pub(crate) total_notes: u64,
    pub(crate) max_tick: u64,
    pub(crate) event_metrics: MidiAnalysisEventMetrics,
    pub(crate) velocity_note_on_counts: Vec<u64>,
    pub(crate) key_note_counts: Vec<u64>,
    pub(crate) pitch_class_note_counts: Vec<u64>,
    pub(crate) track_note_counts: Vec<u64>,
    pub(crate) channel_note_counts: Vec<u64>,
    pub(crate) track_channel_note_counts: Vec<u64>,
    pub(crate) tempo_events: Vec<TempoEventPoint>,
    pub(crate) onset_counts: Vec<(u64, u64)>,
    pub(crate) block_starts: Vec<(u64, u8, u64)>,
    pub(crate) tick_transitions: Vec<TickTransitionPoint>,
    pub(crate) active_deltas: Vec<(u64, i64)>,
    pub(crate) intervals: Vec<NoteIntervalAggregate>,
}

pub(crate) fn extract_analysis(
    parsed: &ParsedMidiFile,
    progress: &(impl Fn(AnalysisProgressUpdate) + Sync),
) -> Result<ExtractedAnalysis, crate::error::MeridianError> {
    let midi = parsed.midi();
    let ppq = midi.ppq();
    if (ppq & 0x8000) != 0 {
        return Err(crate::error::MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    let raw_track_count = midi.track_count();
    let actual_track_count = raw_track_count.max(1);
    progress(AnalysisProgressUpdate {
        progress: 0.0,
        status: format!("Extracting Tracks 0/{actual_track_count}"),
    });
    let finished_tracks = AtomicUsize::new(0);
    let reported_percent = AtomicUsize::new(0);
    let partials = (0..raw_track_count)
        .into_par_iter()
        .map(|track_index| {
            let result = extract_track_analysis(midi, track_index as u32);
            let finished = finished_tracks.fetch_add(1, Ordering::Relaxed) + 1;
            let percent = finished * 100 / actual_track_count;
            let mut last = reported_percent.load(Ordering::Relaxed);
            while percent > last {
                match reported_percent.compare_exchange(
                    last,
                    percent,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        progress(AnalysisProgressUpdate {
                            progress: (finished as f32 / actual_track_count as f32).clamp(0.0, 1.0),
                            status: format!("Extracting Tracks {finished}/{actual_track_count}"),
                        });
                        break;
                    }
                    Err(updated) => last = updated,
                }
            }
            result
        })
        .collect::<Vec<_>>();

    let mut resolved_partials = Vec::with_capacity(partials.len());
    let mut max_tick = 0_u64;
    for partial in partials {
        let partial = partial?;
        max_tick = max_tick.max(partial.max_tick);
        resolved_partials.push(partial);
    }

    let mut total_event_count = 0_u64;
    let mut total_notes = 0_u64;
    let mut event_metrics = MidiAnalysisEventMetrics::default();
    let mut velocity_note_on_counts = vec![0_u64; 128];
    let mut key_note_counts = vec![0_u64; 256];
    let mut pitch_class_note_counts = vec![0_u64; 12];
    let mut track_note_counts = vec![0_u64; actual_track_count];
    let mut channel_note_counts = vec![0_u64; 16];
    let mut track_channel_note_counts = vec![0_u64; actual_track_count * 16];
    let mut tempo_events = Vec::new();
    let mut onset_counts = FxHashMap::<u64, u64>::default();
    let mut block_starts = FxHashMap::<(u64, u8), u64>::default();
    let mut active_deltas = FxHashMap::<u64, i64>::default();
    let mut intervals = FxHashMap::<(u64, u64), u64>::default();
    let mut tick_transitions = Vec::new();

    for partial in resolved_partials {
        total_event_count += partial.total_event_count;
        total_notes += partial.total_notes;
        accumulate_event_metrics(&mut event_metrics, &partial.event_metrics);
        for (index, count) in partial.velocity_note_on_counts.into_iter().enumerate() {
            velocity_note_on_counts[index] += count;
        }
        for (index, count) in partial.key_note_counts.into_iter().enumerate() {
            key_note_counts[index] += count;
        }
        for (index, count) in partial.pitch_class_note_counts.into_iter().enumerate() {
            pitch_class_note_counts[index] += count;
        }
        let track_index = partial.track_index as usize;
        if let Some(slot) = track_note_counts.get_mut(track_index) {
            *slot += partial.total_notes;
        }
        for (index, count) in partial.channel_note_counts.into_iter().enumerate() {
            channel_note_counts[index] += count;
        }
        for (index, count) in partial.track_channel_note_counts.into_iter().enumerate() {
            let target = track_index * 16 + index;
            if let Some(slot) = track_channel_note_counts.get_mut(target) {
                *slot += count;
            }
        }
        tempo_events.extend(partial.tempo_events);
        for (tick, count) in partial.onset_counts {
            *onset_counts.entry(tick).or_insert(0) += count;
        }
        for (tick, key, count) in partial.block_starts {
            *block_starts.entry((tick, key)).or_insert(0) += count;
        }
        for transition in partial.tick_transitions {
            *active_deltas.entry(transition.tick).or_insert(0) += transition.net_delta;
            tick_transitions.push(transition);
        }
        for interval in partial.intervals {
            *intervals
                .entry((interval.start_tick, interval.end_tick))
                .or_insert(0) += interval.count;
        }
        for (start_tick, count) in partial.unresolved_starts {
            *active_deltas.entry(max_tick).or_insert(0) -= count as i64;
            *intervals.entry((start_tick, max_tick)).or_insert(0) += count;
        }
    }

    tempo_events.par_sort_unstable_by_key(|event| (event.tick, event.track, event.event_index));
    tick_transitions.par_sort_unstable_by_key(|transition| (transition.tick, transition.track));

    let mut onset_counts = onset_counts.into_iter().collect::<Vec<_>>();
    onset_counts.par_sort_unstable_by_key(|(tick, _)| *tick);

    let mut block_starts = block_starts
        .into_iter()
        .map(|((tick, key), count)| (tick, key, count))
        .collect::<Vec<_>>();
    block_starts.par_sort_unstable_by_key(|(tick, key, _)| (*tick, *key));

    let mut active_deltas = active_deltas
        .into_iter()
        .filter(|(_, delta)| *delta != 0)
        .collect::<Vec<_>>();
    active_deltas.par_sort_unstable_by_key(|(tick, _)| *tick);

    let mut intervals = intervals
        .into_iter()
        .map(|((start_tick, end_tick), count)| NoteIntervalAggregate {
            start_tick,
            end_tick,
            count,
        })
        .collect::<Vec<_>>();
    intervals.par_sort_unstable_by_key(|interval| (interval.start_tick, interval.end_tick));

    Ok(ExtractedAnalysis {
        ppq,
        actual_track_count,
        total_event_count,
        total_notes,
        max_tick,
        event_metrics,
        velocity_note_on_counts,
        key_note_counts,
        pitch_class_note_counts,
        track_note_counts,
        channel_note_counts,
        track_channel_note_counts,
        tempo_events,
        onset_counts,
        block_starts,
        tick_transitions,
        active_deltas,
        intervals,
    })
}

fn extract_track_analysis(
    midi: &ToolkitMidiFile,
    track_index: u32,
) -> Result<TrackAnalysisPartial, crate::error::MeridianError> {
    let mut absolute_tick = 0_u64;
    let mut event_index = 0_u32;
    let mut total_event_count = 0_u64;
    let mut total_notes = 0_u64;
    let mut max_tick = 0_u64;
    let mut event_metrics = MidiAnalysisEventMetrics::default();
    let mut velocity_note_on_counts = [0_u64; 128];
    let mut key_note_counts = [0_u64; 256];
    let mut pitch_class_note_counts = [0_u64; 12];
    let mut channel_note_counts = [0_u64; 16];
    let mut track_channel_note_counts = [0_u64; 16];
    let mut tempo_events = Vec::new();
    let mut onset_counts = FxHashMap::<u64, u64>::default();
    let mut block_starts = FxHashMap::<(u64, u8), u64>::default();
    let mut interval_counts = FxHashMap::<(u64, u64), u64>::default();
    let mut unresolved_starts = FxHashMap::<u64, u64>::default();
    let mut open_notes = FxHashMap::<(u8, u8), VecDeque<u64>>::default();
    let mut tick_transitions = Vec::new();
    let mut current_transition_tick = None;
    let mut current_transition_net = 0_i64;
    let mut current_transition_peak = 0_i64;
    let mut current_transition_running = 0_i64;

    let track = midi.iter_track(track_index).ok_or_else(|| {
        crate::error::MeridianError::InvalidMidi(format!("missing track {}", track_index))
    })?;

    for event in track {
        let event = event.map_err(|e| crate::error::MeridianError::MidiLoad(format!("{e:?}")))?;
        absolute_tick = absolute_tick.saturating_add(event.delta);
        max_tick = max_tick.max(absolute_tick);
        total_event_count += 1;

        match event.as_event() {
            Event::NoteOn(note_on) => {
                event_metrics.note_on_events += 1;
                velocity_note_on_counts[note_on.velocity as usize] += 1;
                if note_on.velocity == 0 {
                    event_metrics.zero_velocity_note_on_events += 1;
                    if close_note_interval(
                        &mut open_notes,
                        &mut interval_counts,
                        note_on.key,
                        note_on.channel,
                        absolute_tick,
                    ) {
                        record_tick_transition(
                            &mut tick_transitions,
                            &mut current_transition_tick,
                            &mut current_transition_net,
                            &mut current_transition_peak,
                            &mut current_transition_running,
                            absolute_tick,
                            -1,
                            track_index,
                        );
                    }
                } else {
                    total_notes += 1;
                    key_note_counts[note_on.key as usize] += 1;
                    pitch_class_note_counts[note_on.key as usize % 12] += 1;
                    channel_note_counts[note_on.channel as usize] += 1;
                    track_channel_note_counts[note_on.channel as usize] += 1;
                    *onset_counts.entry(absolute_tick).or_insert(0) += 1;
                    *block_starts
                        .entry((absolute_tick, note_on.key))
                        .or_insert(0) += 1;
                    open_notes
                        .entry((note_on.key, note_on.channel))
                        .or_default()
                        .push_back(absolute_tick);
                    record_tick_transition(
                        &mut tick_transitions,
                        &mut current_transition_tick,
                        &mut current_transition_net,
                        &mut current_transition_peak,
                        &mut current_transition_running,
                        absolute_tick,
                        1,
                        track_index,
                    );
                }
            }
            Event::NoteOff(note_off) => {
                event_metrics.note_off_events += 1;
                if close_note_interval(
                    &mut open_notes,
                    &mut interval_counts,
                    note_off.key,
                    note_off.channel,
                    absolute_tick,
                ) {
                    record_tick_transition(
                        &mut tick_transitions,
                        &mut current_transition_tick,
                        &mut current_transition_net,
                        &mut current_transition_peak,
                        &mut current_transition_running,
                        absolute_tick,
                        -1,
                        track_index,
                    );
                }
            }
            Event::ProgramChange(_) => event_metrics.program_change_events += 1,
            Event::ControlChange(_) => event_metrics.control_change_events += 1,
            Event::PitchWheelChange(_) => event_metrics.pitch_bend_events += 1,
            Event::ChannelPressure(_) => event_metrics.channel_pressure_events += 1,
            Event::PolyphonicKeyPressure(_) => event_metrics.polyphonic_pressure_events += 1,
            Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_) => {
                event_metrics.sysex_events += 1;
            }
            Event::Text(text) => {
                event_metrics.text_events += 1;
                match text.kind {
                    TextEventKind::Lyric => event_metrics.lyric_events += 1,
                    TextEventKind::Marker => event_metrics.marker_events += 1,
                    TextEventKind::CuePoint => event_metrics.cue_point_events += 1,
                    TextEventKind::TrackName => event_metrics.track_name_events += 1,
                    TextEventKind::InstrumentName => event_metrics.instrument_name_events += 1,
                    _ => {}
                }
            }
            Event::Tempo(tempo) => {
                event_metrics.tempo_events += 1;
                tempo_events.push(TempoEventPoint {
                    tick: absolute_tick,
                    track: track_index,
                    event_index,
                    micros_per_quarter: tempo.tempo.max(1),
                });
            }
            Event::TimeSignature(_) => event_metrics.time_signature_events += 1,
            Event::KeySignature(_) => event_metrics.key_signature_events += 1,
            _ => {}
        }

        event_index = event_index.saturating_add(1);
    }

    flush_tick_transition(
        &mut tick_transitions,
        &mut current_transition_tick,
        &mut current_transition_net,
        &mut current_transition_peak,
        &mut current_transition_running,
        track_index,
    );

    for (_, mut starts) in open_notes {
        while let Some(start_tick) = starts.pop_front() {
            *unresolved_starts.entry(start_tick).or_insert(0) += 1;
        }
    }

    Ok(TrackAnalysisPartial {
        track_index,
        total_event_count,
        total_notes,
        max_tick,
        event_metrics,
        velocity_note_on_counts,
        key_note_counts,
        pitch_class_note_counts,
        channel_note_counts,
        track_channel_note_counts,
        tempo_events,
        onset_counts: onset_counts.into_iter().collect(),
        block_starts: block_starts
            .into_iter()
            .map(|((tick, key), count)| (tick, key, count))
            .collect(),
        tick_transitions,
        intervals: interval_counts
            .into_iter()
            .map(|((start_tick, end_tick), count)| NoteIntervalAggregate {
                start_tick,
                end_tick,
                count,
            })
            .collect(),
        unresolved_starts: unresolved_starts.into_iter().collect(),
    })
}

fn close_note_interval(
    open_notes: &mut FxHashMap<(u8, u8), VecDeque<u64>>,
    interval_counts: &mut FxHashMap<(u64, u64), u64>,
    key: u8,
    channel: u8,
    end_tick: u64,
) -> bool {
    let Some(queue) = open_notes.get_mut(&(key, channel)) else {
        return false;
    };
    let Some(start_tick) = queue.pop_front() else {
        return false;
    };
    if queue.is_empty() {
        open_notes.remove(&(key, channel));
    }
    *interval_counts.entry((start_tick, end_tick)).or_insert(0) += 1;
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "Tick transition accumulation is easier to follow with the transition scalars passed explicitly."
)]
fn record_tick_transition(
    tick_transitions: &mut Vec<TickTransitionPoint>,
    current_transition_tick: &mut Option<u64>,
    current_transition_net: &mut i64,
    current_transition_peak: &mut i64,
    current_transition_running: &mut i64,
    tick: u64,
    delta: i64,
    track: u32,
) {
    if current_transition_tick
        .map(|current| current != tick)
        .unwrap_or(false)
    {
        flush_tick_transition(
            tick_transitions,
            current_transition_tick,
            current_transition_net,
            current_transition_peak,
            current_transition_running,
            track,
        );
    }

    if current_transition_tick.is_none() {
        *current_transition_tick = Some(tick);
    }

    *current_transition_running += delta;
    *current_transition_peak = (*current_transition_peak).max(*current_transition_running);
    *current_transition_net += delta;
}

fn flush_tick_transition(
    tick_transitions: &mut Vec<TickTransitionPoint>,
    current_transition_tick: &mut Option<u64>,
    current_transition_net: &mut i64,
    current_transition_peak: &mut i64,
    current_transition_running: &mut i64,
    track: u32,
) {
    let Some(tick) = current_transition_tick.take() else {
        return;
    };
    if *current_transition_net != 0 || *current_transition_peak != 0 {
        tick_transitions.push(TickTransitionPoint {
            tick,
            track,
            net_delta: *current_transition_net,
            prefix_peak_delta: *current_transition_peak,
        });
    }
    *current_transition_net = 0;
    *current_transition_peak = 0;
    *current_transition_running = 0;
}

fn accumulate_event_metrics(
    target: &mut MidiAnalysisEventMetrics,
    source: &MidiAnalysisEventMetrics,
) {
    target.note_on_events += source.note_on_events;
    target.note_off_events += source.note_off_events;
    target.zero_velocity_note_on_events += source.zero_velocity_note_on_events;
    target.program_change_events += source.program_change_events;
    target.control_change_events += source.control_change_events;
    target.pitch_bend_events += source.pitch_bend_events;
    target.channel_pressure_events += source.channel_pressure_events;
    target.polyphonic_pressure_events += source.polyphonic_pressure_events;
    target.sysex_events += source.sysex_events;
    target.text_events += source.text_events;
    target.lyric_events += source.lyric_events;
    target.marker_events += source.marker_events;
    target.cue_point_events += source.cue_point_events;
    target.track_name_events += source.track_name_events;
    target.instrument_name_events += source.instrument_name_events;
    target.tempo_events += source.tempo_events;
    target.time_signature_events += source.time_signature_events;
    target.key_signature_events += source.key_signature_events;
}
