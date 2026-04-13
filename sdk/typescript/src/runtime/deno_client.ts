/// <reference lib="deno.ns" />

import { createMeridianClient, createProtocolClient } from "./factories.ts";
import type {
  MeridianClient,
  MeridianProtocolClient,
} from "../internal/client.ts";
import { denoRuntimeAdapter } from "./deno.ts";

/** Create the normal high-level Meridian client for Deno runtimes. */
export async function createDenoMeridianClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianClient> {
  return createMeridianClient({
    executablePath,
    runtime: denoRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}

/** Create the lower-level raw stdio protocol client for Deno runtimes. */
export async function createDenoProtocolClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianProtocolClient> {
  return createProtocolClient({
    executablePath,
    runtime: denoRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}
