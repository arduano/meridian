import type {
  CoreEvent,
  JsonResponse,
  MeridianProtocolCommand,
  ResponseFor,
} from "../../protocol.ts";
import { PROTOCOL_VERSION } from "../../protocol.ts";
import type { MeridianRuntimeAdapter, MeridianSubprocess } from "../runtime.ts";
import {
  type MeridianEventListener,
  MeridianSubprocessError,
} from "./internal.ts";

type PendingRequest = {
  resolve: (response: JsonResponse) => void;
  reject: (error: Error) => void;
};

export interface MeridianProtocolClientOptions {
  executablePath: string;
  runtime: MeridianRuntimeAdapter;
  args?: string[];
}

export class MeridianProtocolClient {
  readonly executablePath: string;
  readonly runtimeName: string;

  static readonly RECENT_EVENT_LIMIT = 256;

  #process: MeridianSubprocess;
  #nextId = 1;
  #pending = new Map<number, PendingRequest>();
  #listeners = new Set<MeridianEventListener>();
  #recentEvents: CoreEvent[] = [];
  #closed = false;

  private constructor(
    executablePath: string,
    runtime: MeridianRuntimeAdapter,
    process: MeridianSubprocess,
  ) {
    this.executablePath = executablePath;
    this.runtimeName = runtime.name;
    this.#process = process;
  }

  static async spawn(
    options: MeridianProtocolClientOptions,
  ): Promise<MeridianProtocolClient> {
    let client: MeridianProtocolClient | null = null;
    const process = await options.runtime.spawn(
      options.executablePath,
      options.args ?? ["stdio"],
      {
        onLine: (line) => {
          if (client) {
            client.#handleLine(line);
          }
        },
        onError: (error) => {
          if (client) {
            client.#handleExit(error);
          }
        },
        onExit: (code, signal) => {
          const suffix = `meridian-cli exited with code=${
            String(code)
          } signal=${String(signal)}`;
          if (client) {
            client.#handleExit(new MeridianSubprocessError(suffix));
          }
        },
      },
    );
    client = new MeridianProtocolClient(
      options.executablePath,
      options.runtime,
      process,
    );
    return client;
  }

  async request<C extends MeridianProtocolCommand>(
    command: C,
  ): Promise<ResponseFor<C>> {
    if (this.#closed) {
      throw new MeridianSubprocessError("Client is closed");
    }

    const id = this.#nextId++;
    const response = await new Promise<JsonResponse>((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      void this.#process.sendLine(
        JSON.stringify({
          protocol_version: PROTOCOL_VERSION,
          id,
          command,
        }),
      ).catch((error) => {
        this.#pending.delete(id);
        reject(error instanceof Error ? error : new Error(String(error)));
      });
    });
    return response.events as ResponseFor<C>;
  }

  onEvent(listener: MeridianEventListener): () => void {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  recentEvents(): readonly CoreEvent[] {
    return this.#recentEvents;
  }

  async close(): Promise<void> {
    if (this.#closed) {
      return;
    }
    try {
      await this.request({ type: "shutdown" });
    } catch {
      // Best effort; the process may already be exiting.
    } finally {
      this.#closed = true;
      await this.#process.kill();
    }
  }

  #handleLine(line: string): void {
    let response: JsonResponse;
    try {
      response = JSON.parse(line) as JsonResponse;
    } catch (error) {
      this.#handleExit(
        error instanceof Error
          ? error
          : new Error(`Invalid JSON response: ${String(error)}`),
      );
      return;
    }

    if (response.id === null) {
      for (const event of response.events) {
        this.#recentEvents.push(event);
        if (
          this.#recentEvents.length > MeridianProtocolClient.RECENT_EVENT_LIMIT
        ) {
          this.#recentEvents.shift();
        }
        for (const listener of this.#listeners) {
          listener(event);
        }
      }
      return;
    }

    const pending = this.#pending.get(response.id);
    if (!pending) {
      return;
    }
    this.#pending.delete(response.id);
    pending.resolve(response);
  }

  #handleExit(error: Error): void {
    if (this.#closed && this.#pending.size === 0) {
      return;
    }
    this.#closed = true;
    for (const [id, pending] of this.#pending) {
      this.#pending.delete(id);
      pending.reject(error);
    }
  }
}
