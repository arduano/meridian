/// <reference lib="deno.ns" />

import {
  createMeridianClient,
  createProtocolClient,
} from "./factories.ts";
import type {
  MeridianClient,
  MeridianProtocolClient,
} from "../internal/client.ts";
import { denoRuntimeAdapter } from "./deno.ts";

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
