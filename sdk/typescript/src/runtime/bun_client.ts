import { createMeridianClient, createProtocolClient } from "../index.ts";
import type {
  MeridianClient,
  MeridianProtocolClient,
} from "../internal/client.ts";
import { bunRuntimeAdapter } from "./bun.ts";

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
