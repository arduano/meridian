import type {
  MeridianRuntimeAdapter,
  MeridianSubprocess,
  SpawnHandlers,
} from "../internal/runtime.ts";

declare const Bun: any;

const encoder = new TextEncoder();

async function pumpLines(
  stream: ReadableStream<Uint8Array>,
  onLine: (line: string) => void,
): Promise<void> {
  const reader = stream.pipeThrough(new TextDecoderStream()).getReader();
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
  } finally {
    reader.releaseLock();
  }
}

class BunMeridianSubprocess implements MeridianSubprocess {
  #subprocess: any;
  #writeChain = Promise.resolve();

  constructor(subprocess: any) {
    this.#subprocess = subprocess;
  }

  sendLine(line: string): Promise<void> {
    this.#writeChain = this.#writeChain.then(() =>
      this.#subprocess.stdin.write(encoder.encode(`${line}\n`)).then(() => {})
    );
    return this.#writeChain;
  }

  async kill(): Promise<void> {
    try {
      this.#subprocess.stdin.end();
    } catch {
      // ignore
    }
    this.#subprocess.kill();
  }
}

export const bunRuntimeAdapter: MeridianRuntimeAdapter = {
  name: "bun",
  async spawn(
    executablePath: string,
    args: string[],
    handlers: SpawnHandlers,
  ): Promise<MeridianSubprocess> {
    const subprocess = Bun.spawn({
      cmd: [executablePath, ...args],
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });
    void pumpLines(subprocess.stdout, handlers.onLine).catch((error) => {
      handlers.onError(
        error instanceof Error ? error : new Error(String(error)),
      );
    });
    void subprocess.exited.then((code) => {
      handlers.onExit(code, null);
    }).catch((error) => {
      handlers.onError(
        error instanceof Error ? error : new Error(String(error)),
      );
    });
    return new BunMeridianSubprocess(subprocess);
  },
};
