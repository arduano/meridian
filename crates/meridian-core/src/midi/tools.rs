use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use super::modifier_tools::{
    change_ppq::ChangePpqTool,
    channel_remap::{ChannelMapEntry, ChannelRemapTool},
    control_change::{ControlChangeTool, ControlValue, ControllerMapEntry, ControllerScaleEntry},
    extract_track::ExtractTrackTool,
    humanize::{HumanizeCollisionMode, HumanizeTool},
    key_map::{KeyMapEntry, KeyMapTool, KeyRange},
    meta_text::{MetaTextTool, TextKind},
    note_length::NoteLengthTool,
    pitch_bend::PitchBendTool,
    program::{ChannelProgram, ProgramTool},
    quantize::{QuantizeMode, QuantizeTool},
    range_select::{RangeEdgeBehavior, RangeSelectTool},
    shared_metadata_track::SharedMetadataTrackTool,
    sysex::SysexTool,
    tempo_map::{TempoMapTool, TempoPoint},
    time_warp::{TimeWarpPoint, TimeWarpTool},
    track_route::{TrackMapEntry, TrackRouteTool},
    velocity_map::{VelocityMapTool, VelocityPoint},
};

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
