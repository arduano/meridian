export * from "./protocol.ts";
export * from "./helpers.ts";
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
  type MidiModificationOptions,
  MidiProcessJobHandle,
  MidiProcessTask,
  type MidiToolTaskOptions,
  type StartAnalysisForFileOptions,
  VideoRenderJobHandle,
  type VideoRenderOptions,
  VideoRenderTask,
} from "./internal/client.ts";

import { MeridianClient, MeridianProtocolClient } from "./internal/client.ts";
import type { MeridianRuntimeAdapter } from "./internal/runtime.ts";

export interface CreateClientOptions {
  executablePath: string;
  runtime: MeridianRuntimeAdapter;
  args?: string[];
}

export async function createProtocolClient(
  options: CreateClientOptions,
): Promise<MeridianProtocolClient> {
  return MeridianProtocolClient.spawn(options);
}

export async function createMeridianClient(
  options: CreateClientOptions,
): Promise<MeridianClient> {
  const protocol = await createProtocolClient(options);
  return new MeridianClient(protocol);
}
