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
    shared_metadata_track::{SharedMetadataTrackDestination, SharedMetadataTrackTool},
    sysex::SysexTool,
    tempo_map::{TempoMapDestination, TempoMapTool, TempoPoint},
    time_warp::{TimeWarpPoint, TimeWarpTool},
    track_route::{TrackMapEntry, TrackRouteTool},
    velocity_map::{VelocityMapTool, VelocityPoint},
};

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
    Quantize(QuantizeTool),
    Humanize(HumanizeTool),
    KeyMap(KeyMapTool),
    MetaText(MetaTextTool),
    Sysex(SysexTool),
    SharedMetadataTrack(SharedMetadataTrackTool),
}
