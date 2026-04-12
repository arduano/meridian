// Shared constructor logic behind the public SDK entrypoints.
// Runtime-specific wrappers call into this module so the package root can
// re-export a single canonical client-creation surface.
import type { MeridianRuntimeAdapter } from "../internal/runtime.ts";
import { MeridianClient, MeridianProtocolClient } from "../internal/client.ts";

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
