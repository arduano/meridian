/// <reference lib="deno.ns" />

import type {
  MeridianRuntimeAdapter,
  MeridianSubprocess,
  SpawnHandlers,
} from "../internal/runtime.ts";

const encoder = new TextEncoder();

async function pumpLines(
  stream: ReadableStream<Uint8Array>,
  onLine: (line: string) => void,
): Promise<void> {
  const reader = (stream as ReadableStream<BufferSource>)
    .pipeThrough(new TextDecoderStream())
    .getReader();
  let buffer = "";
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      buffer += value;
      while (true) {
        const newlineIndex = buffer.indexOf("\n");
        if (newlineIndex < 0) {
          break;
        }
        const line = buffer.slice(0, newlineIndex).trim();
        buffer = buffer.slice(newlineIndex + 1);
        if (line.length > 0) {
          onLine(line);
        }
      }
    }
    const trailing = buffer.trim();
    if (trailing.length > 0) {
      onLine(trailing);
    }
  } finally {
    reader.releaseLock();
  }
}

class DenoMeridianSubprocess implements MeridianSubprocess {
  #child: Deno.ChildProcess;
  #stdin: WritableStreamDefaultWriter<Uint8Array>;
  #stdoutPump: Promise<void>;
  #stderrPump: Promise<void>;
  #statusPump: Promise<void>;
  #writeChain = Promise.resolve();

  constructor(
    child: Deno.ChildProcess,
    stdoutPump: Promise<void>,
    stderrPump: Promise<void>,
    statusPump: Promise<void>,
  ) {
    this.#child = child;
    this.#stdin = child.stdin.getWriter();
    this.#stdoutPump = stdoutPump;
    this.#stderrPump = stderrPump;
    this.#statusPump = statusPump;
  }

  sendLine(line: string): Promise<void> {
    this.#writeChain = this.#writeChain.then(() =>
      this.#stdin.write(encoder.encode(`${line}\n`))
    );
    return this.#writeChain;
  }

  async kill(): Promise<void> {
    try {
      await this.#stdin.close();
    } catch {
      // ignore
    }
    try {
      this.#stdin.releaseLock();
    } catch {
      // ignore
    }
    try {
      this.#child.kill();
    } catch {
      // ignore
    }
    await Promise.allSettled([
      this.#stdoutPump,
      this.#stderrPump,
      this.#statusPump,
    ]);
  }
}

export const denoRuntimeAdapter: MeridianRuntimeAdapter = {
  name: "deno",
  async spawn(
    executablePath: string,
    args: string[],
    handlers: SpawnHandlers,
  ): Promise<MeridianSubprocess> {
    const child = new Deno.Command(executablePath, {
      args,
      stdin: "piped",
      stdout: "piped",
      stderr: "piped",
    }).spawn();

    const stdoutPump = pumpLines(child.stdout, handlers.onLine).catch((error) => {
      handlers.onError(
        error instanceof Error ? error : new Error(String(error)),
      );
    });
    const stderrPump = child.stderr.pipeTo(new WritableStream<Uint8Array>()).catch(() => {});
    const statusPump = child.status.then((status: Deno.CommandStatus) => {
      handlers.onExit(status.code, status.signal ?? null);
    }).catch((error: unknown) => {
      handlers.onError(
        error instanceof Error ? error : new Error(String(error)),
      );
    });

    return new DenoMeridianSubprocess(
      child,
      stdoutPump,
      stderrPump,
      statusPump,
    );
  },
};
