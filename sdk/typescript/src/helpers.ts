import type {
  ChangePpqTool,
  ChannelRemapTool,
  ControlChangeTool,
  ExtractTrackTool,
  HumanizeTool,
  KeyMapTool,
  MetaTextTool,
  MidiModifierTool,
  NoteLengthTool,
  PitchBendTool,
  ProgramTool,
  QuantizeTool,
  RangeSelectTool,
  SharedMetadataTrackTool,
  SysexTool,
  TempoMapDestination,
  TempoMapTool,
  TempoPoint,
  TimeWarpTool,
  TrackMapEntry,
  TrackRouteTool,
  VelocityMapTool,
  VelocityPoint,
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

function withDefaults<T extends MidiModifierTool>(value: T): T {
  return structuredClone(value);
}

export const midiTools = {
  rangeSelect(config: Partial<RangeSelectTool> = {}): RangeSelectTool {
    return withDefaults({
      tool: "range_select",
      start_ticks: 0,
      end_ticks: 1,
      offset_ticks: null,
      track_select: null,
      preserve_system_events: false,
      edge_behavior: "trim",
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
    collapseAll(): TrackRouteTool {
      return { tool: "track_route", mode: "collapse_all" };
    },
    splitByChannel(): TrackRouteTool {
      return { tool: "track_route", mode: "split_by_channel" };
    },
    map(mappings: TrackMapEntry[]): TrackRouteTool {
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
    polyline(points: VelocityPoint[]): VelocityMapTool {
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
  sharedMetadataTrack(
    config: Partial<SharedMetadataTrackTool> = {},
  ): SharedMetadataTrackTool {
    return withDefaults({
      tool: "shared_metadata_track",
      destination: { mode: "create_new" },
      strip_redundant_events: false,
      move_tempo_events: true,
      move_time_signatures: true,
      move_key_signatures: true,
      move_text_events: true,
      move_unknown_meta_events: false,
      move_channel_prefix_events: false,
      move_midi_port_events: false,
      move_control_change_events: false,
      move_program_change_events: false,
      move_pitch_bend_events: false,
      move_channel_pressure_events: false,
      ...config,
    });
  },
};
