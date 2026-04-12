import type { DeepPartial } from "../../helpers.ts";
import type {
  AudioRenderOptions,
  SaveFrameOptions,
  MidiAnalysisOptions,
  MidiToolTaskOptions,
  StartAnalysisForFileOptions,
  VideoRenderAudioOptions,
  VideoRenderOptions,
} from "./options.ts";
import type {
  ImageExportConfig,
  MidiAnalysisKind,
  MidiFilesMergeConfig,
  ProtocolVideoRenderConfig,
  SdkAudioRenderConfig,
} from "../../protocol.ts";
import { deepMerge } from "../../helpers.ts";
import { inferRendererFromScene, MeridianSubprocessError, uniqueKinds } from "./internal.ts";

export const DEFAULT_ANALYSIS_KINDS: MidiAnalysisKind[] = [
  "file",
  "summary",
  "events",
  "notes",
  "tempo",
];

export function normalizeAnalysisOptions(
  midiPath: string,
  options: MidiAnalysisOptions = {},
): StartAnalysisForFileOptions {
  const hasExplicitKinds = options.file === true ||
    options.summary === true ||
    options.events === true ||
    options.notes === true ||
    options.tempo === true;
  const kinds: MidiAnalysisKind[] = hasExplicitKinds
    ? []
    : [...DEFAULT_ANALYSIS_KINDS];
  if (options.file) kinds.push("file");
  if (options.summary) kinds.push("summary");
  if (options.events) kinds.push("events");
  if (options.notes) kinds.push("notes");
  if (options.tempo) kinds.push("tempo");

  let bucketCount: number | null = null;
  if (options.buckets === true) {
    kinds.push("buckets");
    bucketCount = 256;
  } else if (typeof options.buckets === "number") {
    kinds.push("buckets");
    bucketCount = options.buckets;
  }

  return {
    midiPath,
    kinds: uniqueKinds(kinds),
    bucketCount,
    ...(options.onProgress ? { onProgress: options.onProgress } : {}),
  };
}

const DEFAULT_IMAGE_EXPORT_CONFIG: ImageExportConfig = {
  color_mode: "premultiplied",
  export_premultiplied_rgb: false,
  export_straight_rgb: false,
  export_alpha_mask: false,
};

export function normalizeMidiToolOptions(
  options: MidiToolTaskOptions,
): MidiToolTaskOptions {
  return {
    input: options.input,
    output: options.output,
    tool: structuredClone(options.tool),
    ...(options.onEvent ? { onEvent: options.onEvent } : {}),
  };
}

export function normalizeAudioRenderOptions(
  options: AudioRenderOptions,
): AudioRenderOptions {
  return {
    midiPath: options.midiPath,
    output: options.output,
    sampleRate: options.sampleRate ?? null,
    channels: options.channels ?? null,
    useLimiter: options.useLimiter ?? null,
    ffmpegArgs: options.ffmpegArgs ? [...options.ffmpegArgs] : [],
    soundfonts: options.soundfonts ? [...options.soundfonts] : [],
    ...(options.format !== undefined ? { format: options.format } : {}),
    ...(options.onEvent ? { onEvent: options.onEvent } : {}),
  };
}

export function normalizeVideoRenderOptions(
  options: VideoRenderOptions,
): VideoRenderOptions {
  const renderer = resolveVideoRenderer(options);
  return {
    midiPath: options.midiPath,
    output: options.output,
    startTime: options.startTime ?? null,
    endTime: options.endTime ?? null,
    container: options.container ?? "mp4",
    fps: options.fps,
    width: options.width,
    height: options.height,
    renderer,
    scene: options.scene ? structuredClone(options.scene) : null,
    viewRange: options.viewRange ?? null,
    timeSpace: options.timeSpace ?? null,
    firstKey: options.firstKey ?? null,
    lastKey: options.lastKey ?? null,
    rgbMode: options.rgbMode ?? "premultiplied",
    exportAlphaMask: options.exportAlphaMask ?? false,
    ffmpegArgs: options.ffmpegArgs ? [...options.ffmpegArgs] : [],
    audio: options.audio ? normalizeVideoRenderAudioOptions(options.audio) : null,
    ...(options.onEvent ? { onEvent: options.onEvent } : {}),
  };
}

export function normalizeMidiMergeConfig(
  config: DeepPartial<MidiFilesMergeConfig> | undefined,
): MidiFilesMergeConfig {
  return {
    mode: config?.mode ?? "append_tracks",
    normalize_metadata_track: config?.normalize_metadata_track ?? false,
    ppq_override: config?.ppq_override ?? null,
  };
}

export function toSdkAudioRenderConfig(
  options: AudioRenderOptions,
): SdkAudioRenderConfig {
  return {
    midi_path: options.midiPath,
    output: options.output,
    sample_rate: options.sampleRate ?? null,
    channels: options.channels ?? null,
    use_limiter: options.useLimiter ?? null,
    format: options.format ?? "wav",
    ffmpeg_args: options.ffmpegArgs ?? [],
    soundfonts: options.soundfonts ?? [],
  };
}

export function toProtocolVideoRenderConfig(
  options: VideoRenderOptions,
): ProtocolVideoRenderConfig {
  const renderer = resolveVideoRenderer(options);
  return {
    midi_path: options.midiPath,
    output: options.output,
    start_time: options.startTime ?? null,
    end_time: options.endTime ?? null,
    container: options.container ?? "mp4",
    fps: options.fps,
    width: options.width,
    height: options.height,
    renderer,
    scene: options.scene ? structuredClone(options.scene) : null,
    view_range: options.viewRange ?? null,
    time_space: options.timeSpace ?? null,
    first_key: options.firstKey ?? null,
    last_key: options.lastKey ?? null,
    export: {
      color_mode: options.rgbMode ?? "premultiplied",
      export_alpha_mask: options.exportAlphaMask ?? false,
    },
    ffmpeg_args: options.ffmpegArgs ?? [],
    audio: options.audio ? toProtocolVideoRenderAudioConfig(options.audio) : null,
  };
}

export function normalizeVideoRenderAudioOptions(
  audio: VideoRenderAudioOptions,
): VideoRenderAudioOptions {
  return {
    sampleRate: audio.sampleRate ?? null,
    channels: audio.channels ?? null,
    useLimiter: audio.useLimiter ?? null,
    soundfonts: audio.soundfonts ? [...audio.soundfonts] : [],
    ffmpegArgs: audio.ffmpegArgs ? [...audio.ffmpegArgs] : [],
  };
}

export function toProtocolVideoRenderAudioConfig(
  audio: VideoRenderAudioOptions,
): ProtocolVideoRenderConfig["audio"] {
  return {
    sample_rate: audio.sampleRate ?? null,
    channels: audio.channels ?? null,
    use_limiter: audio.useLimiter ?? null,
    soundfonts: audio.soundfonts ? [...audio.soundfonts] : [],
    ffmpeg_args: audio.ffmpegArgs ? [...audio.ffmpegArgs] : [],
  };
}

export function normalizeSaveFrameOptions(
  options: SaveFrameOptions,
): SaveFrameOptions & { export: ImageExportConfig } {
  return {
    output: options.output,
    ...(options.format !== undefined ? { format: options.format } : {}),
    ...(options.viewportWidth !== undefined
      ? { viewportWidth: options.viewportWidth }
      : {}),
    ...(options.viewportHeight !== undefined
      ? { viewportHeight: options.viewportHeight }
      : {}),
    export: deepMerge(DEFAULT_IMAGE_EXPORT_CONFIG, options.export),
  };
}

function resolveVideoRenderer(
  options: Pick<VideoRenderOptions, "renderer" | "scene">,
): ProtocolVideoRenderConfig["renderer"] {
  if (!options.scene) {
    return options.renderer ?? "pfa";
  }

  const inferred = inferRendererFromScene(options.scene);
  if (options.renderer && options.renderer !== inferred) {
    throw new MeridianSubprocessError(
      `Video render scene config requires renderer ${inferred}, got ${options.renderer}`,
    );
  }
  return inferred;
}
