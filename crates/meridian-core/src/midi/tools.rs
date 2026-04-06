use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use super::modifier_tools::{
    change_ppq::ChangePpqTool,
    channel_remap::{ChannelMapEntry, ChannelRemapTool},
    control_change::{ControlChangeTool, ControlValue, ControllerMapEntry, ControllerScaleEntry},
    extract_track::ExtractTrackTool,
    key_map::{KeyMapEntry, KeyMapTool, KeyRange},
    meta_text::{MetaTextTool, TextKind},
    pitch_bend::PitchBendTool,
    program::{ChannelProgram, ProgramTool},
    sysex::SysexTool,
    tempo_map::{TempoMapTool, TempoPoint},
    velocity_map::{VelocityMapTool, VelocityPoint},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct RangeSelectTool {
    pub reset: bool,
    pub track_min: Option<usize>,
    pub track_max: Option<usize>,
    pub channel_min: Option<u8>,
    pub channel_max: Option<u8>,
    pub key_min: Option<u8>,
    pub key_max: Option<u8>,
    pub velocity_min: Option<u8>,
    pub velocity_max: Option<u8>,
    pub tick_start: Option<u64>,
    pub tick_end: Option<u64>,
    pub event_kinds: Vec<SelectableEventKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
pub enum SelectableEventKind {
    Note,
    Tempo,
    ProgramChange,
    ControlChange,
    PitchBend,
    ChannelPressure,
    PolyphonicPressure,
    Text,
    Sysex,
    MetaOther,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct TimeWarpTool {
    pub points: Vec<TimeWarpPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TimeWarpPoint {
    pub source_tick: u64,
    pub dest_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TrackRouteTool {
    Preserve,
    CollapseAll,
    SplitByChannel,
    Map { mappings: Vec<TrackMapEntry> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct TrackMapEntry {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct NoteLengthTool {
    pub min_ticks: Option<u64>,
    pub max_ticks: Option<u64>,
    pub scale: Option<f32>,
    pub fixed_ticks: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct OverlapRepairTool {
    pub repeated_note_on: RepeatedNoteOnPolicy,
    pub orphan_note_offs: OrphanNoteOffPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum RepeatedNoteOnPolicy {
    Keep,
    #[default]
    ClosePrevious,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum OrphanNoteOffPolicy {
    Keep,
    #[default]
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct QuantizeTool {
    pub grid_ticks: u64,
    pub strength: f32,
    pub quantize_note_ends: bool,
    pub swing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct HumanizeTool {
    pub start_jitter: i64,
    pub length_jitter: i64,
    pub velocity_jitter: i16,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct DedupeTool {
    pub notes: bool,
    pub controls: bool,
    pub tempo: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct MergeBalanceTool {
    pub deconflict_channels: bool,
    pub strip_duplicate_start_state: bool,
    pub prefer_first_tempo_map: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct SharedMetadataTrackTool {
    pub target_track_index: usize,
    pub move_tempo_events: bool,
    pub move_time_signatures: bool,
    pub move_key_signatures: bool,
    pub move_text_events: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct AnalysisGuardTool {
    pub min_note_count: Option<u64>,
    pub max_note_count: Option<u64>,
    pub min_track_count: Option<usize>,
    pub max_track_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum MidiModifierTool {
    RangeSelect(RangeSelectTool),
    TempoMap(TempoMapTool),
    TimeWarp(TimeWarpTool),
    ChannelRemap(ChannelRemapTool),
    TrackRoute(TrackRouteTool),
    Program(ProgramTool),
    ControlChange(ControlChangeTool),
    PitchBend(PitchBendTool),
    VelocityMap(VelocityMapTool),
    ChangePpq(ChangePpqTool),
    ExtractTrack(ExtractTrackTool),
    NoteLength(NoteLengthTool),
    OverlapRepair(OverlapRepairTool),
    Quantize(QuantizeTool),
    Humanize(HumanizeTool),
    KeyMap(KeyMapTool),
    Dedupe(DedupeTool),
    MetaText(MetaTextTool),
    Sysex(SysexTool),
    MergeBalance(MergeBalanceTool),
    SharedMetadataTrack(SharedMetadataTrackTool),
    AnalysisGuard(AnalysisGuardTool),
}
