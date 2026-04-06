import type {
  AnalysisGuardTool,
  ChangePpqTool,
  ChannelRemapTool,
  ControlChangeTool,
  DedupeTool,
  EventFilterConfig,
  ExtractTrackTool,
  FileTimeProcessingConfig,
  HumanizeTool,
  KeyMapTool,
  MergeBalanceTool,
  MetaTextTool,
  MidiFileProcessingConfig,
  MidiModifierTool,
  NoteLengthTool,
  NoteProcessingConfig,
  OverlapRepairTool,
  PitchBendTool,
  PitchProcessingConfig,
  ProgramTool,
  QuantizeTool,
  RangeSelectTool,
  StructureProcessingConfig,
  SysexTool,
  TempoMapDestination,
  TempoMapTool,
  TempoPoint,
  TimeWarpTool,
  TrackRouteTool,
  TrimProcessingConfig,
  VelocityMapTool,
  ZeroVelocityNoteOnMode,
} from "./protocol.ts";

export type DeepPartial<T> = T extends (infer U)[] ? U[]
  : T extends object ? { [K in keyof T]?: DeepPartial<T[K]> }
  : T;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function deepMerge<T>(base: T, override: DeepPartial<T> | undefined): T {
  if (override === undefined) {
    return structuredClone(base);
  }
  if (Array.isArray(base) || Array.isArray(override)) {
    return structuredClone((override ?? base) as T);
  }
  if (!isPlainObject(base) || !isPlainObject(override)) {
    return structuredClone((override ?? base) as T);
  }

  const output: Record<string, unknown> = {};
  const keys = new Set([...Object.keys(base), ...Object.keys(override)]);
  for (const key of keys) {
    const baseValue = (base as Record<string, unknown>)[key];
    const overrideValue = (override as Record<string, unknown>)[key];
    if (overrideValue === undefined) {
      output[key] = structuredClone(baseValue);
      continue;
    }
    output[key] = deepMerge(baseValue, overrideValue as never);
  }
  return output as T;
}

export function defaultEventFilterConfig(): EventFilterConfig {
  return {
    notes: true,
    tempo: true,
    pitch_bend: true,
    channel_controls: true,
    program_changes: true,
    channel_pressure: true,
    polyphonic_pressure: true,
    sysex: true,
    meta_other: true,
  };
}

export function defaultNoteProcessingConfig(): NoteProcessingConfig {
  return {
    min_key: 0,
    max_key: 127,
    transpose: 0,
    velocity_scale: 1,
  };
}

export function defaultTrimProcessingConfig(): TrimProcessingConfig {
  return {
    start_tick: 0,
    end_tick: null,
    inject_edge_state: true,
    close_open_notes_at_end: true,
  };
}

export function defaultFileTimeProcessingConfig(): FileTimeProcessingConfig {
  return {
    offset_ticks: 0,
    ppq_override: null,
    tempo_override: null,
    trim: null,
  };
}

export function defaultPitchProcessingConfig(): PitchProcessingConfig {
  return {
    bend_scale: 1,
    bend_offset: 0,
    min_bend: -8192,
    max_bend: 8191,
  };
}

export function defaultStructureProcessingConfig(): StructureProcessingConfig {
  return {
    split_channels: false,
    collapse_tracks: false,
    remove_empty_tracks: true,
    drop_orphan_note_offs: true,
  };
}

export function defaultMidiFileProcessingConfig(): MidiFileProcessingConfig {
  return {
    time: defaultFileTimeProcessingConfig(),
    notes: defaultNoteProcessingConfig(),
    pitch: defaultPitchProcessingConfig(),
    events: defaultEventFilterConfig(),
    structure: defaultStructureProcessingConfig(),
    tools: [],
    piano_only: false,
    zero_velocity_note_on: "note_off",
  };
}

export function createMidiFileProcessingConfig(
  override?: DeepPartial<MidiFileProcessingConfig>,
): MidiFileProcessingConfig {
  return deepMerge(defaultMidiFileProcessingConfig(), override);
}

function withDefaults<T extends MidiModifierTool>(value: T): T {
  return structuredClone(value);
}

export const midiTools = {
  rangeSelect(config: Partial<RangeSelectTool> = {}): RangeSelectTool {
    return withDefaults({
      tool: "range_select",
      reset: false,
      track_min: null,
      track_max: null,
      channel_min: null,
      channel_max: null,
      key_min: null,
      key_max: null,
      velocity_min: null,
      velocity_max: null,
      tick_start: null,
      tick_end: null,
      event_kinds: [],
      ...config,
    });
  },
  tempoMap: {
    flatten(tempo: number): TempoMapTool {
      return { tool: "tempo_map", mode: "flatten", tempo };
    },
    scaleBpm(factor: number): TempoMapTool {
      return { tool: "tempo_map", mode: "scale_bpm", factor };
    },
    replace(
      points: TempoPoint[],
      destination: TempoMapDestination = "inject_into_first_track",
    ): TempoMapTool {
      return { tool: "tempo_map", mode: "replace", points, destination };
    },
  },
  timeWarp(config: Partial<TimeWarpTool> = {}): TimeWarpTool {
    return withDefaults({ tool: "time_warp", points: [], ...config });
  },
  channelRemap(config: Partial<ChannelRemapTool> = {}): ChannelRemapTool {
    return withDefaults({ tool: "channel_remap", mappings: [], ...config });
  },
  trackRoute: {
    preserve(): TrackRouteTool {
      return { tool: "track_route", mode: "preserve" };
    },
    collapseAll(): TrackRouteTool {
      return { tool: "track_route", mode: "collapse_all" };
    },
    splitByChannel(): TrackRouteTool {
      return { tool: "track_route", mode: "split_by_channel" };
    },
    map(
      mappings: TrackRouteTool extends { mode: "map"; mappings: infer P } ? P
        : never,
    ): TrackRouteTool {
      return { tool: "track_route", mode: "map", mappings };
    },
  },
  program(config: Partial<ProgramTool> = {}): ProgramTool {
    return withDefaults({
      tool: "program",
      force_program: null,
      strip_program_changes: false,
      startup_programs: [],
      ...config,
    });
  },
  controlChange(config: Partial<ControlChangeTool> = {}): ControlChangeTool {
    return withDefaults({
      tool: "control_change",
      strip_controllers: [],
      remap_controllers: [],
      scale_controllers: [],
      inject_start: [],
      ...config,
    });
  },
  pitchBend(config: Partial<PitchBendTool> = {}): PitchBendTool {
    return withDefaults({
      tool: "pitch_bend",
      strip: false,
      scale: 1,
      offset: 0,
      min_bend: -8192,
      max_bend: 8191,
      ...config,
    });
  },
  velocityMap: {
    scale(scale: number): VelocityMapTool {
      return { tool: "velocity_map", mode: "scale", scale };
    },
    gamma(gamma: number): VelocityMapTool {
      return { tool: "velocity_map", mode: "gamma", gamma };
    },
    polyline(
      points: VelocityMapTool extends { mode: "polyline"; points: infer P } ? P
        : never,
    ): VelocityMapTool {
      return { tool: "velocity_map", mode: "polyline", points };
    },
  },
  changePpq(config: { ppq: number }): ChangePpqTool {
    return withDefaults({
      tool: "change_ppq",
      ...config,
    });
  },
  extractTrack(config: { track_index: number }): ExtractTrackTool {
    return withDefaults({
      tool: "extract_track",
      ...config,
    });
  },
  noteLength(config: Partial<NoteLengthTool> = {}): NoteLengthTool {
    return withDefaults({
      tool: "note_length",
      min_ticks: null,
      max_ticks: null,
      scale: null,
      fixed_ticks: null,
      ...config,
    });
  },
  overlapRepair(config: Partial<OverlapRepairTool> = {}): OverlapRepairTool {
    return withDefaults({
      tool: "overlap_repair",
      repeated_note_on: "close_previous",
      orphan_note_offs: "drop",
      ...config,
    });
  },
  quantize(config: Partial<QuantizeTool> = {}): QuantizeTool {
    return withDefaults({
      tool: "quantize",
      rounding_ticks: 1,
      mode: "note_start_only",
      ...config,
    });
  },
  humanize(config: Partial<HumanizeTool> = {}): HumanizeTool {
    return withDefaults({
      tool: "humanize",
      start_jitter: 0,
      length_jitter: 0,
      velocity_jitter: 0,
      seed: 0,
      collision_mode: "stable",
      ...config,
    });
  },
  keyMap(config: Partial<KeyMapTool> = {}): KeyMapTool {
    return withDefaults({
      tool: "key_map",
      mappings: [],
      fold_to_range: null,
      drop_unmapped: false,
      ...config,
    });
  },
  dedupe(config: Partial<DedupeTool> = {}): DedupeTool {
    return withDefaults({
      tool: "dedupe",
      notes: false,
      controls: false,
      tempo: false,
      meta: false,
      ...config,
    });
  },
  metaText(config: Partial<MetaTextTool> = {}): MetaTextTool {
    return withDefaults({
      tool: "meta_text",
      keep_kinds: [],
      ...config,
    });
  },
  sysex(config: Partial<SysexTool> = {}): SysexTool {
    return withDefaults({
      tool: "sysex",
      strip_all: false,
      prepend: [],
      ...config,
    });
  },
  mergeBalance(config: Partial<MergeBalanceTool> = {}): MergeBalanceTool {
    return withDefaults({
      tool: "merge_balance",
      deconflict_channels: false,
      strip_duplicate_start_state: false,
      prefer_first_tempo_map: false,
      ...config,
    });
  },
  analysisGuard(config: Partial<AnalysisGuardTool> = {}): AnalysisGuardTool {
    return withDefaults({
      tool: "analysis_guard",
      min_note_count: null,
      max_note_count: null,
      min_track_count: null,
      max_track_count: null,
      ...config,
    });
  },
};

export function normalizeZeroVelocityNoteOnMode(
  mode?: ZeroVelocityNoteOnMode,
): ZeroVelocityNoteOnMode {
  return mode ?? "note_off";
}
