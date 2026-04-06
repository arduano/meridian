import type { DeepPartial } from "../../helpers.ts";
import type {
  AudioRenderEvent,
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
  kinds?: MidiAnalysisKind[];
  bucketCount?: number | null;
  onProgress?: (progress: { progress: number; status: string }) => void;
}

export interface MidiAnalysisOptions {
  file?: boolean;
  summary?: boolean;
  events?: boolean;
  notes?: boolean;
  tempo?: boolean;
  buckets?: number | boolean | null;
  onProgress?: (progress: { progress: number; status: string }) => void;
}

export interface AudioRenderOptions {
  midiPath: string;
  output: string;
  sampleRate?: number | null;
  channels?: number | null;
  useLimiter?: boolean | null;
  soundfonts?: string[];
  onEvent?: (event: AudioRenderEvent) => void;
}

export interface VideoRenderOptions {
  midiPath: string;
  output: string;
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
  onEvent?: (event: VideoRenderEvent) => void;
}

export interface MidiToolTaskOptions {
  input: string;
  output: string;
  tool: MidiModifierTool;
  onEvent?: (event: MidiProcessEvent) => void;
}

export interface MidiModificationOptions {
  input: string;
  output: string;
  onEvent?: (event: MidiProcessEvent) => void;
}

export interface MidiMergeOptions {
  inputs: string[];
  output: string;
  config?: DeepPartial<MidiFilesMergeConfig>;
}

export type { SdkAudioRenderConfig };
