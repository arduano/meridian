import { createMeridianClient, createProtocolClient } from "./factories.ts";
import type {
  MeridianClient,
  MeridianProtocolClient,
} from "../internal/client.ts";
import { nodeRuntimeAdapter } from "./node.ts";

/** Create the normal high-level Meridian client for Node runtimes. */
export async function createNodeMeridianClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianClient> {
  return createMeridianClient({
    executablePath,
    runtime: nodeRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}

/** Create the lower-level raw stdio protocol client for Node runtimes. */
export async function createNodeProtocolClient(
  executablePath: string,
  args?: string[],
): Promise<MeridianProtocolClient> {
  return createProtocolClient({
    executablePath,
    runtime: nodeRuntimeAdapter,
    ...(args ? { args } : {}),
  });
}
