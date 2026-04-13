// Stable high-level SDK surface shared by the package root and runtime-specific
// wrappers. Callers should normally import these types through `src/index.ts`
// instead of reaching into this file directly.
export {
  MeridianProtocolError,
  MeridianSubprocessError,
} from "./client/internal.ts";
export type { MeridianEventListener } from "./client/internal.ts";
export type { MeridianProtocolClientOptions } from "./client/protocol_client.ts";
export { MeridianProtocolClient } from "./client/protocol_client.ts";
export type {
  AudioRenderOptions,
  MidiAnalysisOptions,
  MidiMergeOptions,
  MidiModificationOptions,
  MidiToolTaskOptions,
  SaveFrameOptions,
  StartAnalysisForFileOptions,
  VideoRenderAudioOptions,
  VideoRenderOptions,
} from "./client/options.ts";
export type { AnalysisProgress } from "./client/handles.ts";
export {
  AudioRenderJobHandle,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
  VideoRenderJobHandle,
} from "./client/handles.ts";
export {
  AudioRenderTask,
  MidiAnalysisTask,
  MidiProcessTask,
  VideoRenderTask,
} from "./client/tasks.ts";
export { MeridianClient } from "./client/meridian_client.ts";
