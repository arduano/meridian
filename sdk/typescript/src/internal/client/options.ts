import type { DeepPartial } from "../../helpers.ts";
import type {
  AudioRenderEvent,
  ImageExportConfig,
  ImageOutputFormat,
  MidiAnalysisKind,
  MidiFilesMergeConfig,
  MidiModifierTool,
  MidiProcessEvent,
  ProtocolVideoRenderConfig,
  SdkAudioRenderConfig,
  VideoRenderEvent,
} from "../../protocol.ts";

export interface StartAnalysisForFileOptions {
  midiPath: string;
  kinds: MidiAnalysisKind[];
  bucketCount: number | null;
  onProgress?: (progress: { progress: number; status: string }) => void;
}

/** Declarative analysis options for `client.analysis(...)`. */
export interface MidiAnalysisOptions {
  file?: boolean;
  summary?: boolean;
  events?: boolean;
  notes?: boolean;
  tempo?: boolean;
  buckets?: number | boolean | null;
  onProgress?: (progress: { progress: number; status: string }) => void;
}

/** Offline audio render options for `client.audio.render(...)`. */
export interface AudioRenderOptions {
  midiPath: string;
  output: string;
  sampleRate?: number | null;
  channels?: number | null;
  useLimiter?: boolean | null;
  format?: SdkAudioRenderConfig["format"];
  ffmpegArgs?: string[];
  soundfonts?: string[];
  onEvent?: (event: AudioRenderEvent) => void;
}

/** Offline video render options for `client.video.render(...)`. */
export interface VideoRenderOptions {
  midiPath: string;
  output: string;
  startTime?: number | null;
  endTime?: number | null;
  container?: ProtocolVideoRenderConfig["container"];
  fps: number;
  width: number;
  height: number;
  renderer?: ProtocolVideoRenderConfig["renderer"];
  scene?: ProtocolVideoRenderConfig["scene"];
  viewRange?: number | null;
  timeSpace?: ProtocolVideoRenderConfig["time_space"];
  firstKey?: number | null;
  lastKey?: number | null;
  rgbMode?: ProtocolVideoRenderConfig["export"]["color_mode"];
  exportAlphaMask?: boolean | null;
  ffmpegArgs?: string[];
  audio?: VideoRenderAudioOptions | null;
  onEvent?: (event: VideoRenderEvent) => void;
}

/** Optional muxed-audio overrides for a video render. */
export interface VideoRenderAudioOptions {
  sampleRate?: number | null;
  channels?: number | null;
  useLimiter?: boolean | null;
  soundfonts?: string[];
  ffmpegArgs?: string[];
}

/** Fully-specified process task options used by the low-level MIDI task API. */
export interface MidiToolTaskOptions {
  input: string;
  output: string;
  tool: MidiModifierTool;
  onEvent?: (event: MidiProcessEvent) => void;
}

/** Shared input/output options for the modifier convenience helpers. */
export interface MidiModificationOptions {
  input: string;
  output: string;
  onEvent?: (event: MidiProcessEvent) => void;
}

/** Merge options for `client.merge.midiFiles(...)`. */
export interface MidiMergeOptions {
  inputs: string[];
  output: string;
  config?: DeepPartial<MidiFilesMergeConfig>;
}

/** Save-frame options for `client.display.saveFrame(...)`. */
export interface SaveFrameOptions {
  output: string;
  format?: ImageOutputFormat;
  viewportWidth?: number;
  viewportHeight?: number;
  export?: DeepPartial<ImageExportConfig>;
}

export type { SdkAudioRenderConfig };
