// Public SDK surface:
// - protocol.ts: generated schema re-exports plus handwritten SDK aliases
// - helpers.ts: builder utilities
// - runtime/*-client.ts: runtime-specific entrypoints for the package root
export * from "./protocol.ts";
export * from "./helpers.ts";
export {
  createMeridianClient,
  createProtocolClient,
  type CreateClientOptions,
} from "./runtime/factories.ts";
export {
  createDenoMeridianClient,
  createDenoProtocolClient,
} from "./runtime/deno_client.ts";
export {
  createNodeMeridianClient,
  createNodeProtocolClient,
} from "./runtime/node_client.ts";
export {
  createBunMeridianClient,
  createBunProtocolClient,
} from "./runtime/bun_client.ts";
export {
  AudioRenderJobHandle,
  type AudioRenderOptions,
  AudioRenderTask,
  MeridianClient,
  MeridianProtocolClient,
  MeridianProtocolError,
  MeridianSubprocessError,
  MidiAnalysisJobHandle,
  type MidiAnalysisOptions,
  type MidiMergeOptions,
  MidiAnalysisTask,
  type MidiModificationOptions,
  MidiProcessJobHandle,
  MidiProcessTask,
  type SaveFrameOptions,
  type MidiToolTaskOptions,
  type StartAnalysisForFileOptions,
  type VideoRenderAudioOptions,
  VideoRenderJobHandle,
  type VideoRenderOptions,
  VideoRenderTask,
} from "./internal/client.ts";
