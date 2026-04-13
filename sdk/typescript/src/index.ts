/**
 * Meridian TypeScript SDK package root.
 *
 * Start here as a caller:
 * - use `createDenoMeridianClient`, `createNodeMeridianClient`, or
 *   `createBunMeridianClient` for the normal high-level SDK
 * - use `create*ProtocolClient` when you want raw stdio protocol access
 * - use `midiTools` and the protocol exports when you need typed config
 *   builders and schema-level types
 */
export * from "./protocol.ts";
export * from "./helpers.ts";
export {
  type CreateClientOptions,
  createMeridianClient,
  createProtocolClient,
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
  MidiAnalysisTask,
  type MidiMergeOptions,
  type MidiModificationOptions,
  MidiProcessJobHandle,
  MidiProcessTask,
  type MidiToolTaskOptions,
  type SaveFrameOptions,
  type StartAnalysisForFileOptions,
  type VideoRenderAudioOptions,
  VideoRenderJobHandle,
  type VideoRenderOptions,
  VideoRenderTask,
} from "./internal/client.ts";
