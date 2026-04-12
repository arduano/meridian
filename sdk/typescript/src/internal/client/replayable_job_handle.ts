import type { CoreEvent } from "../../protocol.ts";
import type { MeridianProtocolClient } from "./protocol_client.ts";
import { MeridianSubprocessError } from "./internal.ts";

export abstract class ReplayableJobHandle<
  TStatus extends { state: string },
  TEvent extends { type: string },
  TFinishedEvent extends TEvent,
> {
  readonly jobId: number;
  protected status: TStatus;
  protected readonly protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #eventListeners = new Set<(event: TEvent) => void>();
  #eventHistory: TEvent[] = [];
  #settled = false;
  #done: Promise<TFinishedEvent>;
  #resolve!: (value: TFinishedEvent) => void;
  #reject!: (error: Error) => void;

  protected constructor(
    protocol: MeridianProtocolClient,
    initialStatus: TStatus & { job_id: number },
  ) {
    this.protocol = protocol;
    this.status = initialStatus;
    this.jobId = initialStatus.job_id;
    this.#done = new Promise<TFinishedEvent>((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
  }

  protected startTracking(): void {
    this.#unsubscribe = this.protocol.onEvent((event) =>
      this.handleCoreEvent(event)
    );
    for (const event of this.protocol.recentEvents()) {
      this.handleCoreEvent(event);
      if (this.#settled) {
        break;
      }
    }
  }

  protected abstract handleCoreEvent(event: CoreEvent): void;

  protected isSettled(): boolean {
    return this.#settled;
  }

  protected updateStatus(status: TStatus): void {
    this.status = status;
  }

  protected emitEvent(event: TEvent): void {
    this.#eventHistory.push(event);
    for (const listener of this.#eventListeners) {
      listener(event);
    }
  }

  protected finish(event: TFinishedEvent): void {
    this.#settled = true;
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    this.#resolve(event);
  }

  protected fail(message: string): void {
    this.#settled = true;
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    this.#reject(new MeridianSubprocessError(message));
  }

  onEvent(listener: (event: TEvent) => void): () => void {
    this.#eventListeners.add(listener);
    for (const event of this.#eventHistory) {
      listener(event);
    }
    return () => {
      this.#eventListeners.delete(listener);
    };
  }

  wait(): Promise<TFinishedEvent> {
    return this.#done;
  }
}
