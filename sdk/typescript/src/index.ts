export * from "./protocol.ts";
export * from "./helpers.ts";
export {
  type MidiAnalysisOptions,
  type AudioRenderOptions,
  type MidiModificationOptions,
  type MidiToolTaskOptions,
  type StartAnalysisForFileOptions,
  MeridianClient,
  AudioRenderJobHandle,
  AudioRenderTask,
  MidiAnalysisTask,
  MidiProcessTask,
  MeridianProtocolClient,
  MeridianProtocolError,
  MeridianSubprocessError,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
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
