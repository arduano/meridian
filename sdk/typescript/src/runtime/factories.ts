// Shared constructor logic behind the public SDK entrypoints.
// Runtime-specific wrappers call into this module so the package root can
// re-export a single canonical client-creation surface.
import type { MeridianRuntimeAdapter } from "../internal/runtime.ts";
import { MeridianClient, MeridianProtocolClient } from "../internal/client.ts";

/** Options shared by every runtime-specific Meridian client factory. */
export interface CreateClientOptions {
  executablePath: string;
  runtime: MeridianRuntimeAdapter;
  args?: string[];
}

/** Spawn a raw stdio protocol client for advanced command/event workflows. */
export async function createProtocolClient(
  options: CreateClientOptions,
): Promise<MeridianProtocolClient> {
  return MeridianProtocolClient.spawn(options);
}

/** Spawn the normal high-level Meridian SDK client. */
export async function createMeridianClient(
  options: CreateClientOptions,
): Promise<MeridianClient> {
  const protocol = await createProtocolClient(options);
  return new MeridianClient(protocol);
}
