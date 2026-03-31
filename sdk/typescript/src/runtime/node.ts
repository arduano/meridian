import { spawn } from "node:child_process";
import * as readline from "node:readline";

import type {
  MeridianRuntimeAdapter,
  MeridianSubprocess,
  SpawnHandlers,
} from "../internal/runtime.ts";

class NodeMeridianSubprocess implements MeridianSubprocess {
  #child: ReturnType<typeof spawn>;

  constructor(child: ReturnType<typeof spawn>) {
    this.#child = child;
  }

  async sendLine(line: string): Promise<void> {
    await new Promise<void>((resolve, reject) => {
      this.#child.stdin.write(`${line}\n`, (error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve();
      });
    });
  }

  async kill(): Promise<void> {
    this.#child.stdin.end();
    this.#child.kill();
  }
}

export const nodeRuntimeAdapter: MeridianRuntimeAdapter = {
  name: "node",
  async spawn(
    executablePath: string,
    args: string[],
    handlers: SpawnHandlers,
  ): Promise<MeridianSubprocess> {
    const child = spawn(executablePath, args, {
      stdio: ["pipe", "pipe", "pipe"],
    });
    const lineReader = readline.createInterface({ input: child.stdout });
    lineReader.on("line", (line) => {
      const trimmed = line.trim();
      if (trimmed.length > 0) {
        handlers.onLine(trimmed);
      }
    });
    child.on("error", (error) => handlers.onError(error));
    child.on("exit", (code, signal) => handlers.onExit(code, signal));
    return new NodeMeridianSubprocess(child);
  },
};
