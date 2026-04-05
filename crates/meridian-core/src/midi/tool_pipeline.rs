use std::collections::{HashMap, HashSet, VecDeque};

use midi_toolkit::{
    events::{
        ControlChangeEvent, Event, MIDIEvent, NoteOffEvent, SystemExclusiveMessageEvent,
        TempoEvent, TextEventKind,
    },
    sequence::event::Delta,
};

use crate::error::MeridianError;

use super::tools::{
    AnalysisGuardTool, ChannelRemapTool, ControlChangeTool, DedupeTool, KeyMapTool, KeyRange,
    MergeBalanceTool, MetaTextTool, MidiModifierTool, NoteLengthTool, OrphanNoteOffPolicy,
    OverlapRepairTool, PitchBendTool, ProgramTool, QuantizeTool, RangeSelectTool,
    RepeatedNoteOnPolicy, SelectableEventKind, SharedMetadataTrackTool, SysexTool, TempoMapTool,
    TextKind, TimeWarpPoint, TimeWarpTool, TrackRouteTool, VelocityMapTool,
};

#[derive(Debug, Clone)]
struct AbsoluteEvent {
    tick: u64,
    order: u64,
    event: Event,
}

#[derive(Debug, Clone, Default)]
struct SelectionFilter {
    track_min: Option<usize>,
    track_max: Option<usize>,
    channel_min: Option<u8>,
    channel_max: Option<u8>,
    key_min: Option<u8>,
    key_max: Option<u8>,
    velocity_min: Option<u8>,
    velocity_max: Option<u8>,
    tick_start: Option<u64>,
    tick_end: Option<u64>,
    event_kinds: Vec<SelectableEventKind>,
}

pub fn apply_modifier_tools(
    tracks: &mut Vec<Vec<Delta<u64, Event>>>,
    tools: &[MidiModifierTool],
) -> Result<(), MeridianError> {
    if tools.is_empty() {
        return Ok(());
    }
    let mut abs_tracks = tracks_to_absolute(tracks);
    let mut selection = SelectionFilter::default();

    for tool in tools {
        match tool {
            MidiModifierTool::RangeSelect(tool) => selection = update_selection(selection, tool),
            MidiModifierTool::TempoMap(tool) => {
                apply_tempo_map_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::TimeWarp(tool) => {
                apply_time_warp_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::ChannelRemap(tool) => {
                apply_channel_remap_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::TrackRoute(tool) => apply_track_route_tool(&mut abs_tracks, tool),
            MidiModifierTool::Program(tool) => {
                apply_program_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::ControlChange(tool) => {
                apply_control_change_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::PitchBend(tool) => {
                apply_pitch_bend_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::VelocityMap(tool) => {
                apply_velocity_map_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::NoteLength(tool) => {
                apply_note_length_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::OverlapRepair(tool) => {
                apply_overlap_repair_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::Quantize(tool) => {
                apply_quantize_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::Humanize(tool) => {
                apply_humanize_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::KeyMap(tool) => apply_key_map_tool(&mut abs_tracks, &selection, tool),
            MidiModifierTool::Dedupe(tool) => apply_dedupe_tool(&mut abs_tracks, tool),
            MidiModifierTool::MetaText(tool) => {
                apply_meta_text_tool(&mut abs_tracks, &selection, tool)
            }
            MidiModifierTool::Sysex(tool) => apply_sysex_tool(&mut abs_tracks, &selection, tool),
            MidiModifierTool::MergeBalance(tool) => apply_merge_balance_tool(&mut abs_tracks, tool),
            MidiModifierTool::SharedMetadataTrack(tool) => {
                apply_shared_metadata_track_tool(&mut abs_tracks, tool)
            }
            MidiModifierTool::AnalysisGuard(tool) => apply_analysis_guard_tool(&abs_tracks, tool)?,
        }
    }

    *tracks = absolute_to_tracks(abs_tracks);
    Ok(())
}

fn tracks_to_absolute(tracks: &[Vec<Delta<u64, Event>>]) -> Vec<Vec<AbsoluteEvent>> {
    tracks
        .iter()
        .map(|track| {
            let mut tick = 0u64;
            track
                .iter()
                .enumerate()
                .map(|(order, event)| {
                    tick = tick.saturating_add(event.delta);
                    AbsoluteEvent {
                        tick,
                        order: order as u64,
                        event: event.event.clone(),
                    }
                })
                .collect()
        })
        .collect()
}

fn absolute_to_tracks(mut tracks: Vec<Vec<AbsoluteEvent>>) -> Vec<Vec<Delta<u64, Event>>> {
    tracks
        .iter_mut()
        .map(|track| {
            track.sort_by(|a, b| a.tick.cmp(&b.tick).then(a.order.cmp(&b.order)));
            let mut previous_tick = 0u64;
            track
                .iter()
                .map(|event| {
                    let delta = event.tick.saturating_sub(previous_tick);
                    previous_tick = event.tick;
                    Delta::new(delta, event.event.clone())
                })
                .collect()
        })
        .collect()
}

fn update_selection(mut selection: SelectionFilter, tool: &RangeSelectTool) -> SelectionFilter {
    if tool.reset {
        selection = SelectionFilter::default();
    }
    selection.track_min = tool.track_min.or(selection.track_min);
    selection.track_max = tool.track_max.or(selection.track_max);
    selection.channel_min = tool.channel_min.or(selection.channel_min);
    selection.channel_max = tool.channel_max.or(selection.channel_max);
    selection.key_min = tool.key_min.or(selection.key_min);
    selection.key_max = tool.key_max.or(selection.key_max);
    selection.velocity_min = tool.velocity_min.or(selection.velocity_min);
    selection.velocity_max = tool.velocity_max.or(selection.velocity_max);
    selection.tick_start = tool.tick_start.or(selection.tick_start);
    selection.tick_end = tool.tick_end.or(selection.tick_end);
    if !tool.event_kinds.is_empty() {
        selection.event_kinds = tool.event_kinds.clone();
    }
    selection
}

fn event_selected(track: usize, event: &AbsoluteEvent, selection: &SelectionFilter) -> bool {
    if selection.track_min.is_some_and(|min| track < min) {
        return false;
    }
    if selection.track_max.is_some_and(|max| track > max) {
        return false;
    }
    if selection.tick_start.is_some_and(|start| event.tick < start) {
        return false;
    }
    if selection.tick_end.is_some_and(|end| event.tick > end) {
        return false;
    }
    if selection.channel_min.is_some() || selection.channel_max.is_some() {
        let Some(channel) = event.event.channel() else {
            return false;
        };
        if selection.channel_min.is_some_and(|min| channel < min) {
            return false;
        }
        if selection.channel_max.is_some_and(|max| channel > max) {
            return false;
        }
    }
    if selection.key_min.is_some() || selection.key_max.is_some() {
        let Some(key) = event.event.key() else {
            return false;
        };
        if selection.key_min.is_some_and(|min| key < min) {
            return false;
        }
        if selection.key_max.is_some_and(|max| key > max) {
            return false;
        }
    }
    if selection.velocity_min.is_some() || selection.velocity_max.is_some() {
        let Some(velocity) = event_velocity(&event.event) else {
            return false;
        };
        if selection.velocity_min.is_some_and(|min| velocity < min) {
            return false;
        }
        if selection.velocity_max.is_some_and(|max| velocity > max) {
            return false;
        }
    }
    if !selection.event_kinds.is_empty()
        && !selection
            .event_kinds
            .iter()
            .any(|kind| event_matches_kind(&event.event, kind))
    {
        return false;
    }
    true
}

fn event_matches_kind(event: &Event, kind: &SelectableEventKind) -> bool {
    match kind {
        SelectableEventKind::Note => matches!(event, Event::NoteOn(_) | Event::NoteOff(_)),
        SelectableEventKind::Tempo => matches!(event, Event::Tempo(_)),
        SelectableEventKind::ProgramChange => matches!(event, Event::ProgramChange(_)),
        SelectableEventKind::ControlChange => matches!(event, Event::ControlChange(_)),
        SelectableEventKind::PitchBend => matches!(event, Event::PitchWheelChange(_)),
        SelectableEventKind::ChannelPressure => matches!(event, Event::ChannelPressure(_)),
        SelectableEventKind::PolyphonicPressure => matches!(event, Event::PolyphonicKeyPressure(_)),
        SelectableEventKind::Text => matches!(event, Event::Text(_)),
        SelectableEventKind::Sysex => matches!(
            event,
            Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_)
        ),
        SelectableEventKind::MetaOther => matches!(
            event,
            Event::UnknownMeta(_)
                | Event::ChannelPrefix(_)
                | Event::MIDIPort(_)
                | Event::SMPTEOffset(_)
                | Event::TimeSignature(_)
                | Event::KeySignature(_)
        ),
    }
}

fn event_velocity(event: &Event) -> Option<u8> {
    match event {
        Event::NoteOn(note) => Some(note.velocity),
        Event::PolyphonicKeyPressure(note) => Some(note.velocity),
        _ => None,
    }
}

fn apply_tempo_map_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &TempoMapTool,
) {
    match tool {
        TempoMapTool::Flatten { tempo } => {
            for (track_index, track) in tracks.iter_mut().enumerate() {
                track.retain(|event| {
                    !(matches!(event.event, Event::Tempo(_))
                        && event_selected(track_index, event, selection))
                });
            }
            ensure_track_exists(tracks);
            tracks[0].push(AbsoluteEvent {
                tick: 0,
                order: 0,
                event: Event::Tempo(Box::new(TempoEvent { tempo: *tempo })),
            });
        }
        TempoMapTool::ScaleBpm { factor } => {
            let safe_factor = factor.max(0.0001);
            for (track_index, track) in tracks.iter_mut().enumerate() {
                for event in track {
                    if event_selected(track_index, event, selection) {
                        if let Event::Tempo(tempo) = &mut event.event {
                            tempo.tempo = ((tempo.tempo as f64 / safe_factor).round() as i64)
                                .clamp(1, 0xFFFFFF)
                                as u32;
                        }
                    }
                }
            }
        }
        TempoMapTool::Replace { points } => {
            for track in tracks.iter_mut() {
                track.retain(|event| !matches!(event.event, Event::Tempo(_)));
            }
            ensure_track_exists(tracks);
            for (index, point) in points.iter().enumerate() {
                tracks[0].push(AbsoluteEvent {
                    tick: point.tick,
                    order: index as u64,
                    event: Event::Tempo(Box::new(TempoEvent { tempo: point.tempo })),
                });
            }
        }
    }
}

fn apply_time_warp_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &TimeWarpTool,
) {
    if tool.points.len() < 2 {
        return;
    }
    let mut points = tool.points.clone();
    points.sort_by_key(|point| point.source_tick);
    for (track_index, track) in tracks.iter_mut().enumerate() {
        for event in track {
            if event_selected(track_index, event, selection) {
                event.tick = map_time_warp_tick(event.tick, &points);
            }
        }
    }
}

fn map_time_warp_tick(tick: u64, points: &[TimeWarpPoint]) -> u64 {
    if tick <= points[0].source_tick {
        return points[0].dest_tick;
    }
    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if tick <= right.source_tick {
            let span = (right.source_tick - left.source_tick).max(1) as f64;
            let t = (tick - left.source_tick) as f64 / span;
            return (left.dest_tick as f64 + (right.dest_tick as f64 - left.dest_tick as f64) * t)
                .round() as u64;
        }
    }
    points.last().map(|point| point.dest_tick).unwrap_or(tick)
}

fn apply_channel_remap_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &ChannelRemapTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        for event in track {
            if !event_selected(track_index, event, selection) {
                continue;
            }
            if let Some(channel) = event.event.channel_mut() {
                if let Some(entry) = tool.mappings.iter().find(|entry| entry.from == *channel) {
                    *channel = entry.to.min(15);
                }
            }
        }
    }
}

fn apply_track_route_tool(tracks: &mut Vec<Vec<AbsoluteEvent>>, tool: &TrackRouteTool) {
    match tool {
        TrackRouteTool::Preserve => {}
        TrackRouteTool::CollapseAll => {
            let merged = tracks.drain(..).flatten().collect::<Vec<_>>();
            *tracks = vec![merged];
        }
        TrackRouteTool::SplitByChannel => {
            let mut routed = vec![Vec::new(); 17];
            for track in tracks.drain(..) {
                for event in track {
                    let index = event
                        .event
                        .channel()
                        .map(|channel| channel as usize + 1)
                        .unwrap_or(0);
                    routed[index].push(event);
                }
            }
            routed.retain(|track| !track.is_empty());
            *tracks = routed;
        }
        TrackRouteTool::Map { mappings } => {
            let mut routed: HashMap<usize, Vec<AbsoluteEvent>> = HashMap::new();
            for (index, track) in tracks.drain(..).enumerate() {
                let target = mappings
                    .iter()
                    .find(|entry| entry.from == index)
                    .map(|entry| entry.to)
                    .unwrap_or(index);
                routed.entry(target).or_default().extend(track);
            }
            let mut keys = routed.keys().copied().collect::<Vec<_>>();
            keys.sort_unstable();
            *tracks = keys
                .into_iter()
                .map(|key| routed.remove(&key).unwrap_or_default())
                .collect();
        }
    }
}

fn apply_program_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &ProgramTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.retain_mut(|event| {
            let selected = event_selected(track_index, event, selection);
            if let Event::ProgramChange(program) = &mut event.event {
                if selected {
                    if tool.keep_only_startup || tool.strip_program_changes {
                        return false;
                    }
                    if let Some(force) = tool.force_program {
                        program.program = force;
                    }
                }
            }
            true
        });
    }
    if !tool.startup_programs.is_empty() {
        ensure_track_exists(tracks);
        for (index, program) in tool.startup_programs.iter().enumerate() {
            tracks[0].push(AbsoluteEvent {
                tick: 0,
                order: index as u64,
                event: Event::ProgramChange(Box::new(midi_toolkit::events::ProgramChangeEvent {
                    channel: program.channel.min(15),
                    program: program.program,
                })),
            });
        }
    }
}

fn apply_control_change_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &ControlChangeTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.retain_mut(|event| {
            let selected = event_selected(track_index, event, selection);
            if let Event::ControlChange(control) = &mut event.event {
                if selected {
                    if tool.strip_controllers.contains(&control.controller) {
                        return false;
                    }
                    if let Some(remap) = tool
                        .remap_controllers
                        .iter()
                        .find(|entry| entry.from == control.controller)
                    {
                        control.controller = remap.to;
                    }
                    if let Some(scale) = tool
                        .scale_controllers
                        .iter()
                        .find(|entry| entry.controller == control.controller)
                    {
                        control.value = ((control.value as f32 * scale.scale).round() as i32)
                            .clamp(0, 127) as u8;
                    }
                }
            }
            true
        });
    }
    if !tool.inject_start.is_empty() {
        ensure_track_exists(tracks);
        for (index, control) in tool.inject_start.iter().enumerate() {
            tracks[0].push(AbsoluteEvent {
                tick: 0,
                order: index as u64,
                event: Event::ControlChange(Box::new(ControlChangeEvent {
                    channel: control.channel.min(15),
                    controller: control.controller,
                    value: control.value,
                })),
            });
        }
    }
}

fn apply_pitch_bend_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &PitchBendTool,
) {
    let mut channels_seen = HashSet::new();
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.retain_mut(|event| {
            let selected = event_selected(track_index, event, selection);
            if let Event::PitchWheelChange(pitch) = &mut event.event {
                channels_seen.insert(pitch.channel);
                if selected {
                    if tool.strip {
                        return false;
                    }
                    let bend =
                        ((pitch.pitch as f32) * tool.scale).round() as i32 + tool.offset as i32;
                    pitch.pitch = bend.clamp(tool.min_bend as i32, tool.max_bend as i32) as i16;
                }
            }
            true
        });
    }
    if tool.reset_at_start && !channels_seen.is_empty() {
        ensure_track_exists(tracks);
        for (index, channel) in channels_seen.into_iter().enumerate() {
            tracks[0].push(AbsoluteEvent {
                tick: 0,
                order: index as u64,
                event: Event::PitchWheelChange(Box::new(
                    midi_toolkit::events::PitchWheelChangeEvent { channel, pitch: 0 },
                )),
            });
        }
    }
}

fn apply_velocity_map_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &VelocityMapTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        for event in track {
            if !event_selected(track_index, event, selection) {
                continue;
            }
            match &mut event.event {
                Event::NoteOn(note) => note.velocity = map_velocity(note.velocity, tool),
                Event::PolyphonicKeyPressure(note) => {
                    note.velocity = map_velocity(note.velocity, tool)
                }
                _ => {}
            }
        }
    }
}

fn map_velocity(value: u8, tool: &VelocityMapTool) -> u8 {
    match tool {
        VelocityMapTool::Scale { scale } => {
            ((value as f32 * scale).round() as i32).clamp(0, 127) as u8
        }
        VelocityMapTool::Gamma { gamma } => {
            (((value as f32 / 127.0).powf(gamma.max(0.0001)) * 127.0).round() as i32).clamp(0, 127)
                as u8
        }
        VelocityMapTool::Polyline { points } => {
            if points.is_empty() {
                return value;
            }
            let mut points = points.clone();
            points.sort_by_key(|point| point.input);
            if value <= points[0].input {
                return points[0].output;
            }
            for window in points.windows(2) {
                let left = &window[0];
                let right = &window[1];
                if value <= right.input {
                    let span = (right.input - left.input).max(1) as f32;
                    let t = (value - left.input) as f32 / span;
                    return (left.output as f32 + (right.output as f32 - left.output as f32) * t)
                        .round()
                        .clamp(0.0, 127.0) as u8;
                }
            }
            points.last().map(|point| point.output).unwrap_or(value)
        }
    }
}

fn apply_note_length_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &NoteLengthTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        let pairs = find_note_pairs(track);
        for (on_index, off_index) in pairs {
            if on_index >= track.len()
                || off_index >= track.len()
                || !event_selected(track_index, &track[on_index], selection)
            {
                continue;
            }
            let start = track[on_index].tick;
            let mut len = track[off_index].tick.saturating_sub(start);
            if let Some(scale) = tool.scale {
                len = ((len as f64 * scale as f64).round() as i64).max(0) as u64;
            }
            if let Some(fixed) = tool.fixed_ticks {
                len = fixed;
            }
            if let Some(min) = tool.min_ticks {
                len = len.max(min);
            }
            if let Some(max) = tool.max_ticks {
                len = len.min(max);
            }
            track[off_index].tick = start.saturating_add(len);
        }
    }
}

fn apply_overlap_repair_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &OverlapRepairTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.sort_by(|a, b| a.tick.cmp(&b.tick).then(a.order.cmp(&b.order)));
        let mut open: HashMap<(u8, u8), VecDeque<usize>> = HashMap::new();
        let mut inserts = Vec::new();
        let mut drops = HashSet::new();

        for index in 0..track.len() {
            match &track[index].event {
                Event::NoteOn(note) if event_selected(track_index, &track[index], selection) => {
                    let key = (note.channel, note.key);
                    if !open.get(&key).map(|queue| queue.is_empty()).unwrap_or(true)
                        && matches!(tool.repeated_note_on, RepeatedNoteOnPolicy::ClosePrevious)
                    {
                        inserts.push(AbsoluteEvent {
                            tick: track[index].tick,
                            order: track[index].order.saturating_sub(1),
                            event: Event::NoteOff(NoteOffEvent {
                                channel: note.channel,
                                key: note.key,
                            }),
                        });
                        open.remove(&key);
                    }
                    open.entry(key).or_default().push_back(index);
                }
                Event::NoteOff(note) if event_selected(track_index, &track[index], selection) => {
                    let key = (note.channel, note.key);
                    match open.get_mut(&key).and_then(|queue| queue.pop_front()) {
                        Some(_) => {}
                        None if matches!(tool.orphan_note_offs, OrphanNoteOffPolicy::Drop) => {
                            drops.insert(index);
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }

        if !drops.is_empty() {
            *track = track
                .iter()
                .enumerate()
                .filter(|(index, _)| !drops.contains(index))
                .map(|(_, event)| event.clone())
                .collect();
        }
        track.extend(inserts);
    }
}

fn apply_quantize_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &QuantizeTool,
) {
    let grid = tool.grid_ticks.max(1);
    let strength = tool.strength.clamp(0.0, 1.0) as f64;
    for (track_index, track) in tracks.iter_mut().enumerate() {
        let pairs = find_note_pairs(track);
        for (on_index, off_index) in pairs {
            if on_index >= track.len() || !event_selected(track_index, &track[on_index], selection)
            {
                continue;
            }
            let quantized_start = quantize_tick(track[on_index].tick, grid, tool.swing);
            track[on_index].tick = lerp_tick(track[on_index].tick, quantized_start, strength);
            if tool.quantize_note_ends && off_index < track.len() {
                let quantized_end = quantize_tick(track[off_index].tick, grid, tool.swing);
                track[off_index].tick = lerp_tick(track[off_index].tick, quantized_end, strength);
            }
        }
    }
}

fn quantize_tick(tick: u64, grid: u64, swing: f32) -> u64 {
    let base = (tick / grid) * grid;
    let next = base.saturating_add(grid);
    let mut target = if tick - base <= next - tick {
        base
    } else {
        next
    };
    if swing.abs() > f32::EPSILON && ((target / grid) % 2 == 1) {
        let swing_offset = (grid as f64 * swing as f64 * 0.5).round() as i64;
        target = apply_signed(target, swing_offset);
    }
    target
}

fn lerp_tick(from: u64, to: u64, strength: f64) -> u64 {
    (from as f64 + (to as f64 - from as f64) * strength)
        .round()
        .max(0.0) as u64
}

fn apply_humanize_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &super::tools::HumanizeTool,
) {
    let mut state = tool.seed;
    for (track_index, track) in tracks.iter_mut().enumerate() {
        let pairs = find_note_pairs(track);
        for (on_index, off_index) in pairs {
            if on_index >= track.len() || !event_selected(track_index, &track[on_index], selection)
            {
                continue;
            }
            let start_jitter = next_jitter(&mut state, tool.start_jitter);
            track[on_index].tick = apply_signed(track[on_index].tick, start_jitter);
            if off_index < track.len() {
                let length_jitter = next_jitter(&mut state, tool.length_jitter);
                track[off_index].tick = apply_signed(track[off_index].tick, length_jitter);
            }
            if let Event::NoteOn(note) = &mut track[on_index].event {
                let velocity_jitter = next_jitter(&mut state, tool.velocity_jitter as i64) as i16;
                note.velocity = ((note.velocity as i16 + velocity_jitter).clamp(1, 127)) as u8;
            }
        }
    }
}

fn next_jitter(state: &mut u64, amount: i64) -> i64 {
    if amount == 0 {
        return 0;
    }
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    let span = amount.unsigned_abs().saturating_mul(2).saturating_add(1);
    (*state % span) as i64 - amount.unsigned_abs() as i64
}

fn apply_key_map_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &KeyMapTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.retain_mut(|event| {
            if !event_selected(track_index, event, selection) {
                return true;
            }
            let Some(key) = event.event.key_mut() else {
                return true;
            };
            if let Some(entry) = tool.mappings.iter().find(|entry| entry.from == *key) {
                *key = entry.to;
                return true;
            }
            if let Some(range) = &tool.fold_to_range {
                *key = fold_key_to_range(*key, range);
                return true;
            }
            !tool.drop_unmapped
        });
    }
}

fn fold_key_to_range(mut key: u8, range: &KeyRange) -> u8 {
    let min = range.min.min(range.max);
    let max = range.max.max(range.min);
    let span = max.saturating_sub(min).saturating_add(1).max(1);
    while key < min {
        key = key.saturating_add(span);
    }
    while key > max {
        key = key.saturating_sub(span);
    }
    key
}

fn apply_dedupe_tool(tracks: &mut Vec<Vec<AbsoluteEvent>>, tool: &DedupeTool) {
    for track in tracks.iter_mut() {
        track.sort_by(|a, b| a.tick.cmp(&b.tick).then(a.order.cmp(&b.order)));
        let mut seen = HashSet::new();
        track.retain(|event| {
            let key = dedupe_key(event, tool);
            if let Some(key) = key {
                seen.insert(key)
            } else {
                true
            }
        });
    }
}

fn dedupe_key(event: &AbsoluteEvent, tool: &DedupeTool) -> Option<String> {
    match &event.event {
        Event::NoteOn(note) if tool.notes => Some(format!(
            "note_on:{}:{}:{}:{}",
            event.tick, note.channel, note.key, note.velocity
        )),
        Event::NoteOff(note) if tool.notes => Some(format!(
            "note_off:{}:{}:{}",
            event.tick, note.channel, note.key
        )),
        Event::ControlChange(control) if tool.controls => Some(format!(
            "cc:{}:{}:{}:{}",
            event.tick, control.channel, control.controller, control.value
        )),
        Event::Tempo(tempo) if tool.tempo => Some(format!("tempo:{}:{}", event.tick, tempo.tempo)),
        Event::Text(text) if tool.meta => Some(format!(
            "text:{}:{}:{:?}",
            event.tick, text.kind as u8, text.bytes
        )),
        _ => None,
    }
}

fn apply_meta_text_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &MetaTextTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        track.retain_mut(|event| {
            let selected = event_selected(track_index, event, selection);
            if let Event::Text(text) = &mut event.event {
                if !selected {
                    return true;
                }
                if tool.strip_all_text {
                    return false;
                }
                if !tool.keep_kinds.is_empty()
                    && !tool
                        .keep_kinds
                        .iter()
                        .any(|kind| text_kind_matches(text.kind, *kind))
                {
                    return false;
                }
                if let Some(prefix) = &tool.prefix_track_names {
                    if text.kind == TextEventKind::TrackName {
                        let mut bytes = prefix.as_bytes().to_vec();
                        bytes.extend_from_slice(&text.bytes);
                        text.bytes = bytes;
                    }
                }
            }
            true
        });
    }
}

fn text_kind_matches(kind: TextEventKind, expected: TextKind) -> bool {
    matches!(
        (kind, expected),
        (TextEventKind::TextEvent, TextKind::Text)
            | (TextEventKind::CopyrightNotice, TextKind::Copyright)
            | (TextEventKind::TrackName, TextKind::TrackName)
            | (TextEventKind::InstrumentName, TextKind::InstrumentName)
            | (TextEventKind::Lyric, TextKind::Lyric)
            | (TextEventKind::Marker, TextKind::Marker)
            | (TextEventKind::CuePoint, TextKind::CuePoint)
            | (TextEventKind::ProgramName, TextKind::ProgramName)
            | (TextEventKind::DeviceName, TextKind::DeviceName)
            | (TextEventKind::Undefined, TextKind::Undefined)
            | (TextEventKind::MetaEvent, TextKind::MetaEvent)
    )
}

fn apply_sysex_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    selection: &SelectionFilter,
    tool: &SysexTool,
) {
    for (track_index, track) in tracks.iter_mut().enumerate() {
        if tool.strip_all {
            track.retain(|event| {
                !(event_selected(track_index, event, selection)
                    && matches!(
                        event.event,
                        Event::SystemExclusiveMessage(_) | Event::EndOfExclusive(_)
                    ))
            });
        }
    }
    if !tool.prepend.is_empty() {
        ensure_track_exists(tracks);
        for (index, data) in tool.prepend.iter().enumerate() {
            tracks[0].push(AbsoluteEvent {
                tick: 0,
                order: index as u64,
                event: Event::SystemExclusiveMessage(Box::new(SystemExclusiveMessageEvent {
                    data: data.clone(),
                })),
            });
        }
    }
}

fn apply_merge_balance_tool(tracks: &mut Vec<Vec<AbsoluteEvent>>, tool: &MergeBalanceTool) {
    if tool.deconflict_channels {
        for (track_index, track) in tracks.iter_mut().enumerate() {
            let target = (track_index % 16) as u8;
            for event in track {
                if let Some(channel) = event.event.channel_mut() {
                    *channel = target;
                }
            }
        }
    }
    if tool.prefer_first_tempo_map {
        for track in tracks.iter_mut().skip(1) {
            track.retain(|event| !matches!(event.event, Event::Tempo(_)));
        }
    }
    if tool.strip_duplicate_start_state {
        for track in tracks.iter_mut() {
            let mut seen = HashSet::new();
            track.retain(|event| {
                if event.tick != 0 {
                    return true;
                }
                let key = dedupe_key(
                    event,
                    &DedupeTool {
                        notes: false,
                        controls: true,
                        tempo: true,
                        meta: true,
                    },
                );
                if let Some(key) = key {
                    seen.insert(key)
                } else {
                    true
                }
            });
        }
    }
}

fn apply_shared_metadata_track_tool(
    tracks: &mut Vec<Vec<AbsoluteEvent>>,
    tool: &SharedMetadataTrackTool,
) {
    let should_move = |event: &Event| match event {
        Event::Tempo(_) => tool.move_tempo_events,
        Event::TimeSignature(_) => tool.move_time_signatures,
        Event::KeySignature(_) => tool.move_key_signatures,
        Event::Text(_) => tool.move_text_events,
        _ => false,
    };
    if !tracks
        .iter()
        .any(|track| track.iter().any(|event| should_move(&event.event)))
    {
        return;
    }

    while tracks.len() <= tool.target_track_index {
        tracks.push(Vec::new());
    }

    let mut moved = Vec::new();
    for (track_index, track) in tracks.iter_mut().enumerate() {
        let mut kept = Vec::with_capacity(track.len());
        for event in track.drain(..) {
            if track_index == tool.target_track_index {
                if should_move(&event.event) {
                    moved.push(event);
                } else {
                    kept.push(event);
                }
            } else if should_move(&event.event) {
                moved.push(event);
            } else {
                kept.push(event);
            }
        }
        *track = kept;
    }

    tracks[tool.target_track_index].extend(moved);
}

fn apply_analysis_guard_tool(
    tracks: &[Vec<AbsoluteEvent>],
    tool: &AnalysisGuardTool,
) -> Result<(), MeridianError> {
    let note_count = tracks
        .iter()
        .flat_map(|track| track.iter())
        .filter(|event| matches!(event.event, Event::NoteOn(_)))
        .count() as u64;
    let track_count = tracks.iter().filter(|track| !track.is_empty()).count();
    if tool.min_note_count.is_some_and(|min| note_count < min) {
        return Err(MeridianError::InvalidMidi(format!(
            "analysis guard failed: note_count {note_count} < {min}",
            min = tool.min_note_count.unwrap()
        )));
    }
    if tool.max_note_count.is_some_and(|max| note_count > max) {
        return Err(MeridianError::InvalidMidi(format!(
            "analysis guard failed: note_count {note_count} > {max}",
            max = tool.max_note_count.unwrap()
        )));
    }
    if tool.min_track_count.is_some_and(|min| track_count < min) {
        return Err(MeridianError::InvalidMidi(format!(
            "analysis guard failed: track_count {track_count} < {min}",
            min = tool.min_track_count.unwrap()
        )));
    }
    if tool.max_track_count.is_some_and(|max| track_count > max) {
        return Err(MeridianError::InvalidMidi(format!(
            "analysis guard failed: track_count {track_count} > {max}",
            max = tool.max_track_count.unwrap()
        )));
    }
    Ok(())
}

fn find_note_pairs(track: &[AbsoluteEvent]) -> Vec<(usize, usize)> {
    let mut open: HashMap<(u8, u8), VecDeque<usize>> = HashMap::new();
    let mut result = Vec::new();
    for (index, event) in track.iter().enumerate() {
        match &event.event {
            Event::NoteOn(note) => open
                .entry((note.channel, note.key))
                .or_default()
                .push_back(index),
            Event::NoteOff(note) => {
                if let Some(on_index) = open
                    .get_mut(&(note.channel, note.key))
                    .and_then(|queue| queue.pop_front())
                {
                    result.push((on_index, index));
                }
            }
            _ => {}
        }
    }
    result
}

fn ensure_track_exists(tracks: &mut Vec<Vec<AbsoluteEvent>>) {
    if tracks.is_empty() {
        tracks.push(Vec::new());
    }
}

fn apply_signed(value: u64, offset: i64) -> u64 {
    if offset >= 0 {
        value.saturating_add(offset as u64)
    } else {
        value.saturating_sub(offset.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    use crate::midi::tools::{
        ChannelMapEntry, ChannelProgram, ControlValue, ControllerMapEntry, ControllerScaleEntry,
        HumanizeTool, KeyMapEntry, SharedMetadataTrackTool, TempoPoint, TextKind, TrackMapEntry,
        VelocityPoint,
    };
    use midi_toolkit::{
        events::{
            ControlChangeEvent, EndOfExclusiveEvent, NoteOnEvent, PitchWheelChangeEvent,
            ProgramChangeEvent, SystemExclusiveMessageEvent, TextEvent, TextEventKind,
        },
        sequence::event::Delta,
    };

    use super::*;

    fn delta(delta: u64, event: Event) -> Delta<u64, Event> {
        Delta::new(delta, event)
    }

    fn absolute_ticks(tracks: &[Vec<Delta<u64, Event>>]) -> Vec<Vec<(u64, Event)>> {
        tracks
            .iter()
            .map(|track| {
                let mut tick = 0u64;
                track
                    .iter()
                    .map(|event| {
                        tick = tick.saturating_add(event.delta);
                        (tick, event.event.clone())
                    })
                    .collect()
            })
            .collect()
    }

    fn note_on(channel: u8, key: u8, velocity: u8) -> Event {
        Event::NoteOn(NoteOnEvent {
            channel,
            key,
            velocity,
        })
    }

    fn note_off(channel: u8, key: u8) -> Event {
        Event::NoteOff(NoteOffEvent { channel, key })
    }

    #[test]
    fn tempo_map_modes_and_time_warp_work() {
        let base_track = vec![
            delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
            delta(10, note_on(0, 60, 100)),
            delta(10, note_off(0, 60)),
            delta(10, Event::Tempo(Box::new(TempoEvent { tempo: 600_000 }))),
        ];

        let mut tracks = vec![base_track.clone()];
        apply_modifier_tools(
            &mut tracks,
            &[MidiModifierTool::TempoMap(TempoMapTool::Flatten {
                tempo: 400_000,
            })],
        )
        .expect("flatten tempo map");
        let abs = absolute_ticks(&tracks);
        let tempos = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::Tempo(tempo) => Some((*tick, tempo.tempo)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(tempos, vec![(0, 400_000)]);

        let mut tracks = vec![base_track.clone()];
        apply_modifier_tools(
            &mut tracks,
            &[MidiModifierTool::TempoMap(TempoMapTool::ScaleBpm {
                factor: 2.0,
            })],
        )
        .expect("scale bpm");
        let abs = absolute_ticks(&tracks);
        let tempos = abs[0]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::Tempo(tempo) => Some(tempo.tempo),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(tempos, vec![250_000, 300_000]);

        let mut tracks = vec![base_track.clone()];
        apply_modifier_tools(
            &mut tracks,
            &[MidiModifierTool::TempoMap(TempoMapTool::Replace {
                points: vec![
                    TempoPoint {
                        tick: 0,
                        tempo: 700_000,
                    },
                    TempoPoint {
                        tick: 24,
                        tempo: 800_000,
                    },
                ],
            })],
        )
        .expect("replace tempo map");
        let abs = absolute_ticks(&tracks);
        let tempos = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::Tempo(tempo) => Some((*tick, tempo.tempo)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(tempos, vec![(0, 700_000), (24, 800_000)]);

        let mut tracks = vec![base_track];
        apply_modifier_tools(
            &mut tracks,
            &[MidiModifierTool::TimeWarp(TimeWarpTool {
                points: vec![
                    TimeWarpPoint {
                        source_tick: 0,
                        dest_tick: 0,
                    },
                    TimeWarpPoint {
                        source_tick: 30,
                        dest_tick: 60,
                    },
                ],
            })],
        )
        .expect("time warp");
        let abs = absolute_ticks(&tracks);
        let note_ticks = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::NoteOn(_) | Event::NoteOff(_) => Some(*tick),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(note_ticks, vec![20, 40]);
    }

    #[test]
    fn selection_routing_and_channel_tools_work() {
        let track_a = vec![
            delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
            delta(5, note_on(0, 60, 90)),
            delta(5, note_off(0, 60)),
        ];
        let track_b = vec![delta(0, note_on(1, 64, 80)), delta(5, note_off(1, 64))];

        let mut tracks = vec![track_a.clone(), track_b.clone()];
        apply_modifier_tools(
            &mut tracks,
            &[
                MidiModifierTool::RangeSelect(RangeSelectTool {
                    channel_min: Some(1),
                    channel_max: Some(1),
                    event_kinds: vec![SelectableEventKind::Note],
                    ..Default::default()
                }),
                MidiModifierTool::ChannelRemap(ChannelRemapTool {
                    mappings: vec![ChannelMapEntry { from: 1, to: 4 }],
                }),
            ],
        )
        .expect("range select + channel remap");
        let abs = absolute_ticks(&tracks);
        let channels = abs
            .into_iter()
            .flatten()
            .filter_map(|(_, event)| event.channel())
            .collect::<Vec<_>>();
        assert!(channels.contains(&0));
        assert!(channels.contains(&4));
        assert!(!channels.contains(&1));

        let mut preserve_tracks = vec![track_a.clone(), track_b.clone()];
        apply_modifier_tools(
            &mut preserve_tracks,
            &[MidiModifierTool::TrackRoute(TrackRouteTool::Preserve)],
        )
        .expect("preserve tracks");
        assert_eq!(preserve_tracks.len(), 2);

        let mut collapse_tracks = vec![track_a.clone(), track_b.clone()];
        apply_modifier_tools(
            &mut collapse_tracks,
            &[MidiModifierTool::TrackRoute(TrackRouteTool::CollapseAll)],
        )
        .expect("collapse tracks");
        assert_eq!(collapse_tracks.len(), 1);

        let mut split_tracks = vec![track_a.clone(), track_b.clone()];
        apply_modifier_tools(
            &mut split_tracks,
            &[MidiModifierTool::TrackRoute(TrackRouteTool::SplitByChannel)],
        )
        .expect("split by channel");
        assert_eq!(split_tracks.len(), 3);

        let mut mapped_tracks = vec![track_a, track_b];
        apply_modifier_tools(
            &mut mapped_tracks,
            &[MidiModifierTool::TrackRoute(TrackRouteTool::Map {
                mappings: vec![
                    TrackMapEntry { from: 0, to: 2 },
                    TrackMapEntry { from: 1, to: 0 },
                ],
            })],
        )
        .expect("map tracks");
        let abs = absolute_ticks(&mapped_tracks);
        assert_eq!(mapped_tracks.len(), 2);
        assert!(matches!(abs[0][0].1, Event::NoteOn(_)));
        assert!(matches!(abs[1][0].1, Event::Tempo(_)));
    }

    #[test]
    fn program_control_and_pitch_tools_work() {
        let mut tracks = vec![vec![
            delta(
                0,
                Event::ProgramChange(Box::new(ProgramChangeEvent {
                    channel: 0,
                    program: 5,
                })),
            ),
            delta(
                0,
                Event::ControlChange(Box::new(ControlChangeEvent {
                    channel: 0,
                    controller: 7,
                    value: 100,
                })),
            ),
            delta(
                0,
                Event::ControlChange(Box::new(ControlChangeEvent {
                    channel: 0,
                    controller: 10,
                    value: 64,
                })),
            ),
            delta(
                10,
                Event::PitchWheelChange(Box::new(PitchWheelChangeEvent {
                    channel: 0,
                    pitch: 300,
                })),
            ),
        ]];

        apply_modifier_tools(
            &mut tracks,
            &[
                MidiModifierTool::Program(ProgramTool {
                    force_program: Some(9),
                    startup_programs: vec![ChannelProgram {
                        channel: 2,
                        program: 12,
                    }],
                    ..Default::default()
                }),
                MidiModifierTool::ControlChange(ControlChangeTool {
                    strip_controllers: vec![10],
                    remap_controllers: vec![ControllerMapEntry { from: 7, to: 11 }],
                    scale_controllers: vec![ControllerScaleEntry {
                        controller: 11,
                        scale: 0.5,
                    }],
                    inject_start: vec![ControlValue {
                        channel: 1,
                        controller: 64,
                        value: 127,
                    }],
                }),
                MidiModifierTool::PitchBend(PitchBendTool {
                    scale: 0.5,
                    offset: 100,
                    min_bend: -200,
                    max_bend: 200,
                    reset_at_start: true,
                    ..Default::default()
                }),
            ],
        )
        .expect("apply state tools");

        let abs = absolute_ticks(&tracks);
        let programs = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::ProgramChange(program) => Some((*tick, program.channel, program.program)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(programs, vec![(0, 0, 9), (0, 2, 12)]);

        let controls = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::ControlChange(control) => {
                    Some((*tick, control.channel, control.controller, control.value))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(controls, vec![(0, 1, 64, 127), (0, 0, 11, 50)]);

        let bends = abs[0]
            .iter()
            .filter_map(|(tick, event)| match event {
                Event::PitchWheelChange(pitch) => Some((*tick, pitch.channel, pitch.pitch)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(bends, vec![(0, 0, 0), (10, 0, 200)]);

        let mut startup_only = vec![vec![delta(
            0,
            Event::ProgramChange(Box::new(ProgramChangeEvent {
                channel: 0,
                program: 3,
            })),
        )]];
        apply_modifier_tools(
            &mut startup_only,
            &[MidiModifierTool::Program(ProgramTool {
                keep_only_startup: true,
                startup_programs: vec![ChannelProgram {
                    channel: 0,
                    program: 8,
                }],
                ..Default::default()
            })],
        )
        .expect("keep startup program only");
        let abs = absolute_ticks(&startup_only);
        let programs = abs[0]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::ProgramChange(program) => Some(program.program),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(programs, vec![8]);
    }

    #[test]
    fn note_transform_tools_work() {
        let mut velocity_scale_tracks = vec![vec![delta(0, note_on(0, 60, 100))]];
        apply_modifier_tools(
            &mut velocity_scale_tracks,
            &[MidiModifierTool::VelocityMap(VelocityMapTool::Scale {
                scale: 0.5,
            })],
        )
        .expect("velocity scale");
        let abs = absolute_ticks(&velocity_scale_tracks);
        assert_eq!(
            abs[0]
                .iter()
                .find_map(|(_, event)| match event {
                    Event::NoteOn(note) => Some(note.velocity),
                    _ => None,
                })
                .unwrap(),
            50
        );

        let mut velocity_gamma_tracks = vec![vec![delta(0, note_on(0, 60, 64))]];
        apply_modifier_tools(
            &mut velocity_gamma_tracks,
            &[MidiModifierTool::VelocityMap(VelocityMapTool::Gamma {
                gamma: 2.0,
            })],
        )
        .expect("velocity gamma");
        let abs = absolute_ticks(&velocity_gamma_tracks);
        let gamma_velocity = abs[0]
            .iter()
            .find_map(|(_, event)| match event {
                Event::NoteOn(note) => Some(note.velocity),
                _ => None,
            })
            .unwrap();
        assert!(gamma_velocity < 64);

        let mut velocity_polyline_tracks = vec![vec![delta(0, note_on(0, 60, 100))]];
        apply_modifier_tools(
            &mut velocity_polyline_tracks,
            &[MidiModifierTool::VelocityMap(VelocityMapTool::Polyline {
                points: vec![
                    VelocityPoint {
                        input: 0,
                        output: 0,
                    },
                    VelocityPoint {
                        input: 127,
                        output: 120,
                    },
                ],
            })],
        )
        .expect("velocity polyline");
        let abs = absolute_ticks(&velocity_polyline_tracks);
        assert_eq!(
            abs[0]
                .iter()
                .find_map(|(_, event)| match event {
                    Event::NoteOn(note) => Some(note.velocity),
                    _ => None,
                })
                .unwrap(),
            94
        );

        let mut note_length_tracks = vec![vec![
            delta(10, note_on(0, 60, 100)),
            delta(10, note_off(0, 60)),
        ]];
        apply_modifier_tools(
            &mut note_length_tracks,
            &[MidiModifierTool::NoteLength(NoteLengthTool {
                fixed_ticks: Some(30),
                ..Default::default()
            })],
        )
        .expect("note length");
        let abs = absolute_ticks(&note_length_tracks);
        let note_ticks = abs[0].iter().map(|(tick, _)| *tick).collect::<Vec<_>>();
        assert_eq!(note_ticks, vec![10, 40]);

        let mut overlap_tracks = vec![vec![
            delta(0, note_on(0, 60, 100)),
            delta(5, note_on(0, 60, 90)),
            delta(5, note_off(0, 60)),
            delta(5, note_off(0, 60)),
        ]];
        apply_modifier_tools(
            &mut overlap_tracks,
            &[MidiModifierTool::OverlapRepair(OverlapRepairTool::default())],
        )
        .expect("overlap repair");
        let abs = absolute_ticks(&overlap_tracks);
        let note_off_ticks = abs[0]
            .iter()
            .filter_map(|(tick, event)| matches!(event, Event::NoteOff(_)).then_some(*tick))
            .collect::<Vec<_>>();
        assert_eq!(note_off_ticks, vec![5, 10]);

        let mut quantize_tracks = vec![vec![
            delta(13, note_on(0, 60, 100)),
            delta(16, note_off(0, 60)),
        ]];
        apply_modifier_tools(
            &mut quantize_tracks,
            &[MidiModifierTool::Quantize(QuantizeTool {
                grid_ticks: 10,
                strength: 1.0,
                quantize_note_ends: true,
                swing: 0.0,
            })],
        )
        .expect("quantize");
        let abs = absolute_ticks(&quantize_tracks);
        let quantized_ticks = abs[0].iter().map(|(tick, _)| *tick).collect::<Vec<_>>();
        assert_eq!(quantized_ticks, vec![10, 30]);

        let mut humanize_tracks = vec![vec![
            delta(10, note_on(0, 60, 100)),
            delta(10, note_off(0, 60)),
        ]];
        let mut state = 1u64;
        let expected_start = apply_signed(10, next_jitter(&mut state, 2));
        let expected_end = apply_signed(20, next_jitter(&mut state, 1));
        let expected_velocity = ((100i16 + next_jitter(&mut state, 5) as i16).clamp(1, 127)) as u8;
        apply_modifier_tools(
            &mut humanize_tracks,
            &[MidiModifierTool::Humanize(HumanizeTool {
                start_jitter: 2,
                length_jitter: 1,
                velocity_jitter: 5,
                seed: 1,
            })],
        )
        .expect("humanize");
        let abs = absolute_ticks(&humanize_tracks);
        let humanized_note_on = abs[0]
            .iter()
            .find_map(|(tick, event)| match event {
                Event::NoteOn(note) => Some((*tick, note.velocity)),
                _ => None,
            })
            .unwrap();
        let note_off = abs[0]
            .iter()
            .find_map(|(tick, event)| matches!(event, Event::NoteOff(_)).then_some(*tick))
            .unwrap();
        assert_eq!(humanized_note_on, (expected_start, expected_velocity));
        assert_eq!(note_off, expected_end);

        let mut key_map_tracks = vec![vec![
            delta(0, note_on(0, 60, 100)),
            delta(0, note_on(0, 85, 100)),
            delta(0, note_on(0, 61, 100)),
        ]];
        apply_modifier_tools(
            &mut key_map_tracks,
            &[MidiModifierTool::KeyMap(KeyMapTool {
                mappings: vec![KeyMapEntry { from: 60, to: 72 }],
                fold_to_range: Some(KeyRange { min: 70, max: 72 }),
                drop_unmapped: true,
            })],
        )
        .expect("key map");
        let abs = absolute_ticks(&key_map_tracks);
        let keys = abs[0]
            .iter()
            .filter_map(|(_, event)| event.key())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec![72, 70, 70]);
    }

    #[test]
    fn dedupe_meta_sysex_merge_and_analysis_tools_work() {
        let mut dedupe_tracks = vec![vec![
            delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
            delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
            delta(
                0,
                Event::ControlChange(Box::new(ControlChangeEvent {
                    channel: 0,
                    controller: 7,
                    value: 100,
                })),
            ),
            delta(
                0,
                Event::ControlChange(Box::new(ControlChangeEvent {
                    channel: 0,
                    controller: 7,
                    value: 100,
                })),
            ),
            delta(10, note_on(0, 60, 100)),
            delta(0, note_on(0, 60, 100)),
            delta(10, note_off(0, 60)),
            delta(0, note_off(0, 60)),
            delta(
                0,
                Event::Text(Box::new(TextEvent {
                    kind: TextEventKind::TrackName,
                    bytes: b"Lead".to_vec(),
                })),
            ),
            delta(
                0,
                Event::Text(Box::new(TextEvent {
                    kind: TextEventKind::TrackName,
                    bytes: b"Lead".to_vec(),
                })),
            ),
        ]];
        apply_modifier_tools(
            &mut dedupe_tracks,
            &[MidiModifierTool::Dedupe(DedupeTool {
                notes: true,
                controls: true,
                tempo: true,
                meta: true,
            })],
        )
        .expect("dedupe");
        let abs = absolute_ticks(&dedupe_tracks);
        assert_eq!(
            abs[0]
                .iter()
                .filter(|(_, event)| matches!(event, Event::Tempo(_)))
                .count(),
            1
        );
        assert_eq!(
            abs[0]
                .iter()
                .filter(|(_, event)| matches!(event, Event::ControlChange(_)))
                .count(),
            1
        );
        assert_eq!(
            abs[0]
                .iter()
                .filter(|(_, event)| matches!(event, Event::NoteOn(_) | Event::NoteOff(_)))
                .count(),
            2
        );
        assert_eq!(
            abs[0]
                .iter()
                .filter(|(_, event)| matches!(event, Event::Text(_)))
                .count(),
            1
        );

        let mut meta_tracks = vec![vec![
            delta(
                0,
                Event::Text(Box::new(TextEvent {
                    kind: TextEventKind::TrackName,
                    bytes: b"Lead".to_vec(),
                })),
            ),
            delta(
                0,
                Event::Text(Box::new(TextEvent {
                    kind: TextEventKind::Lyric,
                    bytes: b"la".to_vec(),
                })),
            ),
        ]];
        apply_modifier_tools(
            &mut meta_tracks,
            &[MidiModifierTool::MetaText(MetaTextTool {
                keep_kinds: vec![TextKind::TrackName],
                prefix_track_names: Some("PFX:".into()),
                ..Default::default()
            })],
        )
        .expect("meta text");
        let abs = absolute_ticks(&meta_tracks);
        let texts = abs[0]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::Text(text) => {
                    Some((text.kind, String::from_utf8_lossy(&text.bytes).into_owned()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(texts, vec![(TextEventKind::TrackName, "PFX:Lead".into())]);

        let mut sysex_tracks = vec![vec![
            delta(
                0,
                Event::SystemExclusiveMessage(Box::new(SystemExclusiveMessageEvent {
                    data: vec![1, 2, 3],
                })),
            ),
            delta(0, Event::EndOfExclusive(Box::new(EndOfExclusiveEvent {}))),
        ]];
        apply_modifier_tools(
            &mut sysex_tracks,
            &[MidiModifierTool::Sysex(SysexTool {
                strip_all: true,
                prepend: vec![vec![9, 9]],
            })],
        )
        .expect("sysex");
        let abs = absolute_ticks(&sysex_tracks);
        let sysex = abs[0]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::SystemExclusiveMessage(msg) => Some(msg.data.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(sysex, vec![vec![9, 9]]);

        let mut shared_metadata_tracks = vec![
            vec![
                delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
                delta(
                    0,
                    Event::Text(Box::new(TextEvent {
                        kind: TextEventKind::TrackName,
                        bytes: b"Conductor".to_vec(),
                    })),
                ),
                delta(10, note_on(0, 60, 100)),
            ],
            vec![
                delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 600_000 }))),
                delta(
                    0,
                    Event::Text(Box::new(TextEvent {
                        kind: TextEventKind::Marker,
                        bytes: b"Verse".to_vec(),
                    })),
                ),
                delta(0, note_on(1, 64, 100)),
            ],
        ];
        apply_modifier_tools(
            &mut shared_metadata_tracks,
            &[MidiModifierTool::SharedMetadataTrack(
                SharedMetadataTrackTool {
                    target_track_index: 0,
                    move_tempo_events: true,
                    move_text_events: true,
                    ..Default::default()
                },
            )],
        )
        .expect("shared metadata track");
        let abs = absolute_ticks(&shared_metadata_tracks);
        assert_eq!(
            abs[0]
                .iter()
                .filter(|(_, event)| matches!(event, Event::Tempo(_) | Event::Text(_)))
                .count(),
            4
        );
        assert_eq!(
            abs[1]
                .iter()
                .filter(|(_, event)| matches!(event, Event::Tempo(_) | Event::Text(_)))
                .count(),
            0
        );

        let mut merge_tracks = vec![
            vec![
                delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
                delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 500_000 }))),
                delta(
                    0,
                    Event::ControlChange(Box::new(ControlChangeEvent {
                        channel: 2,
                        controller: 7,
                        value: 100,
                    })),
                ),
                delta(
                    0,
                    Event::ControlChange(Box::new(ControlChangeEvent {
                        channel: 2,
                        controller: 7,
                        value: 100,
                    })),
                ),
            ],
            vec![
                delta(0, Event::Tempo(Box::new(TempoEvent { tempo: 600_000 }))),
                delta(0, note_on(5, 60, 100)),
            ],
        ];
        apply_modifier_tools(
            &mut merge_tracks,
            &[MidiModifierTool::MergeBalance(MergeBalanceTool {
                deconflict_channels: true,
                strip_duplicate_start_state: true,
                prefer_first_tempo_map: true,
            })],
        )
        .expect("merge balance");
        let abs = absolute_ticks(&merge_tracks);
        let track0_tempo_count = abs[0]
            .iter()
            .filter(|(_, event)| matches!(event, Event::Tempo(_)))
            .count();
        let track1_tempo_count = abs[1]
            .iter()
            .filter(|(_, event)| matches!(event, Event::Tempo(_)))
            .count();
        let track0_cc_channels = abs[0]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::ControlChange(control) => Some(control.channel),
                _ => None,
            })
            .collect::<Vec<_>>();
        let track1_note_channels = abs[1]
            .iter()
            .filter_map(|(_, event)| match event {
                Event::NoteOn(note) => Some(note.channel),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(track0_tempo_count, 1);
        assert_eq!(track1_tempo_count, 0);
        assert_eq!(track0_cc_channels, vec![0]);
        assert_eq!(track1_note_channels, vec![1]);

        let mut guard_tracks = vec![vec![delta(0, note_on(0, 60, 100))]];
        apply_modifier_tools(
            &mut guard_tracks,
            &[MidiModifierTool::AnalysisGuard(AnalysisGuardTool {
                min_note_count: Some(1),
                max_note_count: Some(1),
                min_track_count: Some(1),
                max_track_count: Some(1),
            })],
        )
        .expect("analysis guard passes");
        let err = apply_modifier_tools(
            &mut guard_tracks,
            &[MidiModifierTool::AnalysisGuard(AnalysisGuardTool {
                min_note_count: Some(2),
                ..Default::default()
            })],
        )
        .expect_err("analysis guard should fail");
        assert!(err.to_string().contains("analysis guard failed"));
    }
}
