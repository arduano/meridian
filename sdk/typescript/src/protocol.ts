export const PROTOCOL_VERSION = 1 as const;

export type { AnalysisJobId } from "../generated/AnalysisJobId.ts";
export type { AudioRenderEvent } from "../generated/AudioRenderEvent.ts";
export type { AudioRenderJobId } from "../generated/AudioRenderJobId.ts";
export type { AudioRenderStatus } from "../generated/AudioRenderStatus.ts";
export type { ChannelMapEntry } from "../generated/ChannelMapEntry.ts";
export type { ChannelProgram } from "../generated/ChannelProgram.ts";
export type { ControlValue } from "../generated/ControlValue.ts";
export type { ControllerMapEntry } from "../generated/ControllerMapEntry.ts";
export type { ControllerScaleEntry } from "../generated/ControllerScaleEntry.ts";
export type { CoreErrorCode } from "../generated/CoreErrorCode.ts";
export type { DisplayTimeSpace } from "../generated/DisplayTimeSpace.ts";
export type { DisplayCacheId } from "../generated/DisplayCacheId.ts";
export type { EventFilterConfig } from "../generated/EventFilterConfig.ts";
export type { FileTimeProcessingConfig } from "../generated/FileTimeProcessingConfig.ts";
export type { KeyMapEntry } from "../generated/KeyMapEntry.ts";
export type { KeyRange } from "../generated/KeyRange.ts";
export type { MIDIAnalysisSummary as MidiAnalysisSummary } from "../generated/MIDIAnalysisSummary.ts";
export type { MergeProcessingConfig } from "../generated/MergeProcessingConfig.ts";
export type { MidiAnalysisBucket } from "../generated/MidiAnalysisBucket.ts";
export type { MidiAnalysisData } from "../generated/MidiAnalysisData.ts";
export type { MidiAnalysisEventMetrics } from "../generated/MidiAnalysisEventMetrics.ts";
export type { MidiAnalysisFileMetrics } from "../generated/MidiAnalysisFileMetrics.ts";
export type { MidiAnalysisJobEvent } from "../generated/MidiAnalysisJobEvent.ts";
export type { MidiAnalysisJobStatus } from "../generated/MidiAnalysisJobStatus.ts";
export type { MidiAnalysisKind } from "../generated/MidiAnalysisKind.ts";
export type { MidiAnalysisNoteMetrics } from "../generated/MidiAnalysisNoteMetrics.ts";
export type { MidiAnalysisTempoMetrics } from "../generated/MidiAnalysisTempoMetrics.ts";
export type { MidiFileProcessingConfig } from "../generated/MidiFileProcessingConfig.ts";
export type { MidiFileSelection } from "../generated/MidiFileSelection.ts";
export type { MidiMergeMode } from "../generated/MidiMergeMode.ts";
export type { MidiModifierTool } from "../generated/MidiModifierTool.ts";
export type { MidiProcessEvent } from "../generated/MidiProcessEvent.ts";
export type { MidiProcessJobId } from "../generated/MidiProcessJobId.ts";
export type { MidiProcessStatus } from "../generated/MidiProcessStatus.ts";
export type { NoteProcessingConfig } from "../generated/NoteProcessingConfig.ts";
export type { OrphanNoteOffPolicy } from "../generated/OrphanNoteOffPolicy.ts";
export type { ParsedMidiId } from "../generated/ParsedMidiId.ts";
export type { PitchProcessingConfig } from "../generated/PitchProcessingConfig.ts";
export type { RepeatedNoteOnPolicy } from "../generated/RepeatedNoteOnPolicy.ts";
export type { ProtocolCommand as MeridianProtocolCommand } from "../generated/ProtocolCommand.ts";
export type { ProtocolAudioRenderConfig as SdkAudioRenderConfig } from "../generated/ProtocolAudioRenderConfig.ts";
export type { ProtocolVideoRenderConfig } from "../generated/ProtocolVideoRenderConfig.ts";
export type { ProtocolEvent as CoreEvent } from "../generated/ProtocolEvent.ts";
export type { ProtocolRequest as JsonRequest } from "../generated/ProtocolRequest.ts";
export type { ProtocolResponse as JsonResponse } from "../generated/ProtocolResponse.ts";
export type { RendererKind } from "../generated/RendererKind.ts";
export type { SelectableEventKind } from "../generated/SelectableEventKind.ts";
export type { StructureProcessingConfig } from "../generated/StructureProcessingConfig.ts";
export type { TempoPoint } from "../generated/TempoPoint.ts";
export type { TextKind } from "../generated/TextKind.ts";
export type { TimeWarpPoint } from "../generated/TimeWarpPoint.ts";
export type { TrackMapEntry } from "../generated/TrackMapEntry.ts";
export type { TrimProcessingConfig } from "../generated/TrimProcessingConfig.ts";
export type { VelocityPoint } from "../generated/VelocityPoint.ts";
export type { VideoRenderEvent } from "../generated/VideoRenderEvent.ts";
export type { VideoRenderJobId } from "../generated/VideoRenderJobId.ts";
export type { VideoRenderStatus } from "../generated/VideoRenderStatus.ts";
export type { ZeroVelocityNoteOnMode } from "../generated/ZeroVelocityNoteOnMode.ts";

import type { ProtocolEvent } from "../generated/ProtocolEvent.ts";
import type { ProtocolCommand } from "../generated/ProtocolCommand.ts";
import type { ProtocolRequest } from "../generated/ProtocolRequest.ts";
import type { ProtocolResponse } from "../generated/ProtocolResponse.ts";
import type { AnalysisGuardTool as RawAnalysisGuardTool } from "../generated/AnalysisGuardTool.ts";
import type { ChannelRemapTool as RawChannelRemapTool } from "../generated/ChannelRemapTool.ts";
import type { ControlChangeTool as RawControlChangeTool } from "../generated/ControlChangeTool.ts";
import type { DedupeTool as RawDedupeTool } from "../generated/DedupeTool.ts";
import type { HumanizeTool as RawHumanizeTool } from "../generated/HumanizeTool.ts";
import type { KeyMapTool as RawKeyMapTool } from "../generated/KeyMapTool.ts";
import type { MergeBalanceTool as RawMergeBalanceTool } from "../generated/MergeBalanceTool.ts";
import type { MetaTextTool as RawMetaTextTool } from "../generated/MetaTextTool.ts";
import type { NoteLengthTool as RawNoteLengthTool } from "../generated/NoteLengthTool.ts";
import type { OverlapRepairTool as RawOverlapRepairTool } from "../generated/OverlapRepairTool.ts";
import type { PitchBendTool as RawPitchBendTool } from "../generated/PitchBendTool.ts";
import type { ProgramTool as RawProgramTool } from "../generated/ProgramTool.ts";
import type { QuantizeTool as RawQuantizeTool } from "../generated/QuantizeTool.ts";
import type { RangeSelectTool as RawRangeSelectTool } from "../generated/RangeSelectTool.ts";
import type { SysexTool as RawSysexTool } from "../generated/SysexTool.ts";
import type { TempoMapTool as RawTempoMapTool } from "../generated/TempoMapTool.ts";
import type { TimeWarpTool as RawTimeWarpTool } from "../generated/TimeWarpTool.ts";
import type { TrackRouteTool as RawTrackRouteTool } from "../generated/TrackRouteTool.ts";
import type { VelocityMapTool as RawVelocityMapTool } from "../generated/VelocityMapTool.ts";

export type RangeSelectTool =
  & { tool: "range_select" }
  & RawRangeSelectTool;
export type TempoMapTool = { tool: "tempo_map" } & RawTempoMapTool;
export type TimeWarpTool = { tool: "time_warp" } & RawTimeWarpTool;
export type ChannelRemapTool =
  & { tool: "channel_remap" }
  & RawChannelRemapTool;
export type TrackRouteTool = { tool: "track_route" } & RawTrackRouteTool;
export type ProgramTool = { tool: "program" } & RawProgramTool;
export type ControlChangeTool =
  & { tool: "control_change" }
  & RawControlChangeTool;
export type PitchBendTool = { tool: "pitch_bend" } & RawPitchBendTool;
export type VelocityMapTool = { tool: "velocity_map" } & RawVelocityMapTool;
export type NoteLengthTool = { tool: "note_length" } & RawNoteLengthTool;
export type OverlapRepairTool =
  & { tool: "overlap_repair" }
  & RawOverlapRepairTool;
export type QuantizeTool = { tool: "quantize" } & RawQuantizeTool;
export type HumanizeTool = { tool: "humanize" } & RawHumanizeTool;
export type KeyMapTool = { tool: "key_map" } & RawKeyMapTool;
export type DedupeTool = { tool: "dedupe" } & RawDedupeTool;
export type MetaTextTool = { tool: "meta_text" } & RawMetaTextTool;
export type SysexTool = { tool: "sysex" } & RawSysexTool;
export type MergeBalanceTool =
  & { tool: "merge_balance" }
  & RawMergeBalanceTool;
export type AnalysisGuardTool =
  & { tool: "analysis_guard" }
  & RawAnalysisGuardTool;

export type ErrorEvent = Extract<ProtocolEvent, { type: "error" }>;
export type ParsedMidiLoadedEvent = Extract<
  ProtocolEvent,
  { type: "parsed_midi_loaded" }
>;
export type MidiLoadedEvent = Extract<ProtocolEvent, { type: "midi_loaded" }>;
export type MidiFilesProcessedEvent = Extract<
  ProtocolEvent,
  { type: "midi_files_processed" }
>;
export type MidiAnalysisJobEventWrapper = Extract<
  ProtocolEvent,
  { type: "midi_analysis_job" }
>;
export type MidiAnalysisJobStatusEventWrapper = Extract<
  ProtocolEvent,
  { type: "midi_analysis_job_status" }
>;
export type MidiProcessEventWrapper = Extract<
  ProtocolEvent,
  { type: "midi_process" }
>;
export type MidiProcessStatusEventWrapper = Extract<
  ProtocolEvent,
  { type: "midi_process_status" }
>;
export type AudioRenderEventWrapper = Extract<
  ProtocolEvent,
  { type: "audio_render" }
>;
export type AudioRenderStatusEventWrapper = Extract<
  ProtocolEvent,
  { type: "audio_render_status" }
>;
export type VideoRenderEventWrapper = Extract<
  ProtocolEvent,
  { type: "video_render" }
>;
export type VideoRenderStatusEventWrapper = Extract<
  ProtocolEvent,
  { type: "video_render_status" }
>;
export type ShutdownCompleteEvent = Extract<
  ProtocolEvent,
  { type: "shutdown_complete" }
>;

export type ResponseFor<C extends ProtocolCommand> = C extends
  { type: "load_parsed_midi" } ? [ParsedMidiLoadedEvent] | [ErrorEvent]
  : C extends { type: "load_audio_midi" } ? [MidiLoadedEvent] | [ErrorEvent]
  : C extends { type: "start_midi_analysis_job" }
    ? [MidiAnalysisJobStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "get_midi_analysis_job_status" }
    ? [MidiAnalysisJobStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "start_process_midi_files" }
    ? [MidiProcessStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "cancel_process_midi_files" }
    ? [MidiProcessStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "get_process_midi_status" }
    ? [MidiProcessStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "start_render_audio" }
    ? [AudioRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "cancel_render_audio" }
    ? [AudioRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "get_render_audio_status" }
    ? [AudioRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "start_render_video" }
    ? [VideoRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "cancel_render_video" }
    ? [VideoRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "get_render_video_status" }
    ? [VideoRenderStatusEventWrapper] | [ErrorEvent]
  : C extends { type: "shutdown" } ? [ShutdownCompleteEvent]
  : never;

export type {
  ProtocolCommand,
  ProtocolEvent,
  ProtocolRequest,
  ProtocolResponse,
};
