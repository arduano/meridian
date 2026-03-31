use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TempoMapTool {
    Flatten { tempo: u32 },
    ScaleBpm { factor: f64 },
    Replace { points: Vec<TempoPoint> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TempoPoint {
    pub tick: u64,
    pub tempo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TimeWarpTool {
    pub points: Vec<TimeWarpPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeWarpPoint {
    pub source_tick: u64,
    pub dest_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ChannelRemapTool {
    pub mappings: Vec<ChannelMapEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelMapEntry {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TrackRouteTool {
    Preserve,
    CollapseAll,
    SplitByChannel,
    Map { mappings: Vec<TrackMapEntry> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackMapEntry {
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ProgramTool {
    pub force_program: Option<u8>,
    pub strip_program_changes: bool,
    pub startup_programs: Vec<ChannelProgram>,
    pub keep_only_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelProgram {
    pub channel: u8,
    pub program: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ControlChangeTool {
    pub strip_controllers: Vec<u8>,
    pub remap_controllers: Vec<ControllerMapEntry>,
    pub scale_controllers: Vec<ControllerScaleEntry>,
    pub inject_start: Vec<ControlValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControllerMapEntry {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControllerScaleEntry {
    pub controller: u8,
    pub scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlValue {
    pub channel: u8,
    pub controller: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PitchBendTool {
    pub strip: bool,
    pub scale: f32,
    pub offset: i16,
    pub min_bend: i16,
    pub max_bend: i16,
    pub reset_at_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VelocityMapTool {
    Scale { scale: f32 },
    Gamma { gamma: f32 },
    Polyline { points: Vec<VelocityPoint> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VelocityPoint {
    pub input: u8,
    pub output: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct NoteLengthTool {
    pub min_ticks: Option<u64>,
    pub max_ticks: Option<u64>,
    pub scale: Option<f32>,
    pub fixed_ticks: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct OverlapRepairTool {
    pub repeated_note_on: RepeatedNoteOnPolicy,
    pub orphan_note_offs: OrphanNoteOffPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepeatedNoteOnPolicy {
    Keep,
    #[default]
    ClosePrevious,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrphanNoteOffPolicy {
    Keep,
    #[default]
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct QuantizeTool {
    pub grid_ticks: u64,
    pub strength: f32,
    pub quantize_note_ends: bool,
    pub swing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct HumanizeTool {
    pub start_jitter: i64,
    pub length_jitter: i64,
    pub velocity_jitter: i16,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct KeyMapTool {
    pub mappings: Vec<KeyMapEntry>,
    pub fold_to_range: Option<KeyRange>,
    pub drop_unmapped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyMapEntry {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyRange {
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct DedupeTool {
    pub notes: bool,
    pub controls: bool,
    pub tempo: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct MetaTextTool {
    pub strip_all_text: bool,
    pub keep_kinds: Vec<TextKind>,
    pub prefix_track_names: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    Text,
    Copyright,
    TrackName,
    InstrumentName,
    Lyric,
    Marker,
    CuePoint,
    ProgramName,
    DeviceName,
    Undefined,
    MetaEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SysexTool {
    pub strip_all: bool,
    pub prepend: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct MergeBalanceTool {
    pub deconflict_channels: bool,
    pub strip_duplicate_start_state: bool,
    pub prefer_first_tempo_map: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AnalysisGuardTool {
    pub min_note_count: Option<u64>,
    pub max_note_count: Option<u64>,
    pub min_track_count: Option<usize>,
    pub max_track_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    NoteLength(NoteLengthTool),
    OverlapRepair(OverlapRepairTool),
    Quantize(QuantizeTool),
    Humanize(HumanizeTool),
    KeyMap(KeyMapTool),
    Dedupe(DedupeTool),
    MetaText(MetaTextTool),
    Sysex(SysexTool),
    MergeBalance(MergeBalanceTool),
    AnalysisGuard(AnalysisGuardTool),
}
