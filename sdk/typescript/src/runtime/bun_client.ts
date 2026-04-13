import { createMeridianClient, createProtocolClient } from "./factories.ts";
import type {
  MeridianClient,
  MeridianProtocolClient,
} from "../internal/client.ts";
import { bunRuntimeAdapter } from "./bun.ts";

/** Create the normal high-level Meridian client for Bun runtimes. */
export async function createBunMeridianClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianClient> {
  return createMeridianClient({
    executablePath,
    runtime: bunRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}

/** Create the lower-level raw stdio protocol client for Bun runtimes. */
export async function createBunProtocolClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianProtocolClient> {
  return createProtocolClient({
    executablePath,
    runtime: bunRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}
