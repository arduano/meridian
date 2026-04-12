import type {
  AudioRenderEvent,
  MidiAnalysisData,
  MidiProcessEvent,
  VideoRenderEvent,
} from "../../protocol.ts";
import type { MeridianClient } from "./meridian_client.ts";
import {
  AudioRenderJobHandle,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
  VideoRenderJobHandle,
} from "./handles.ts";
import type {
  AudioRenderOptions,
  MidiAnalysisOptions,
  MidiToolTaskOptions,
  StartAnalysisForFileOptions,
  VideoRenderOptions,
} from "./options.ts";
import { normalizeAnalysisOptions, normalizeAudioRenderOptions, normalizeMidiToolOptions, normalizeVideoRenderOptions } from "./normalizers.ts";

export class MidiAnalysisTask implements PromiseLike<MidiAnalysisData> {
  #client: MeridianClient;
  #spec: StartAnalysisForFileOptions;
  #handlePromise: Promise<MidiAnalysisJobHandle> | null = null;
  #resultPromise: Promise<MidiAnalysisData> | null = null;

  constructor(
    client: MeridianClient,
    midiPath: string,
    options: MidiAnalysisOptions = {},
  ) {
    this.#client = client;
    this.#spec = normalizeAnalysisOptions(midiPath, options);
  }

  start(): Promise<MidiAnalysisJobHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = this.#client.startAnalysis(this.#snapshot());
    }
    return this.#handlePromise;
  }

  then<TResult1 = MidiAnalysisData, TResult2 = never>(
    onfulfilled?:
      | ((value: MidiAnalysisData) => TResult1 | PromiseLike<TResult1>)
      | null,
    onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
  ): Promise<TResult1 | TResult2> {
    if (!this.#resultPromise) {
      this.#resultPromise = this.start().then((handle) => handle.wait());
    }
    return this.#resultPromise.then(onfulfilled, onrejected);
  }

  catch<TResult = never>(
    onrejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
  ): Promise<MidiAnalysisData | TResult> {
    return Promise.resolve(this).catch(onrejected);
  }

  finally(onfinally?: (() => void) | null): Promise<MidiAnalysisData> {
    return Promise.resolve(this).finally(onfinally ?? undefined);
  }

  toJSON(): StartAnalysisForFileOptions {
    return this.#snapshot();
  }

  #snapshot(): StartAnalysisForFileOptions {
    return {
      midiPath: this.#spec.midiPath,
      kinds: [...this.#spec.kinds],
      bucketCount: this.#spec.bucketCount,
      ...(this.#spec.onProgress ? { onProgress: this.#spec.onProgress } : {}),
    };
  }
}

export class MidiProcessTask
  implements
    PromiseLike<Extract<MidiProcessEvent, { type: "process_finished" }>> {
  #client: MeridianClient;
  #options: MidiToolTaskOptions;
  #handlePromise: Promise<MidiProcessJobHandle> | null = null;
  #resultPromise:
    | Promise<Extract<MidiProcessEvent, { type: "process_finished" }>>
    | null = null;

  constructor(client: MeridianClient, options: MidiToolTaskOptions) {
    this.#client = client;
    this.#options = normalizeMidiToolOptions(options);
  }

  start(): Promise<MidiProcessJobHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = this.#client.startMidiProcess(this.#snapshot());
    }
    return this.#handlePromise;
  }

  then<
    TResult1 = Extract<MidiProcessEvent, { type: "process_finished" }>,
    TResult2 = never,
  >(
    onfulfilled?:
      | ((
        value: Extract<MidiProcessEvent, { type: "process_finished" }>,
      ) => TResult1 | PromiseLike<TResult1>)
      | null,
    onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
  ): Promise<TResult1 | TResult2> {
    if (!this.#resultPromise) {
      this.#resultPromise = this.start().then((handle) => handle.wait());
    }
    return this.#resultPromise.then(onfulfilled, onrejected);
  }

  catch<TResult = never>(
    onrejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
  ): Promise<
    Extract<MidiProcessEvent, { type: "process_finished" }> | TResult
  > {
    return Promise.resolve(this).catch(onrejected);
  }

  finally(
    onfinally?: (() => void) | null,
  ): Promise<Extract<MidiProcessEvent, { type: "process_finished" }>> {
    return Promise.resolve(this).finally(onfinally ?? undefined);
  }

  toJSON(): MidiToolTaskOptions {
    return this.#snapshot();
  }

  #snapshot(): MidiToolTaskOptions {
    return normalizeMidiToolOptions(this.#options);
  }
}

export class AudioRenderTask
  implements
    PromiseLike<Extract<AudioRenderEvent, { type: "render_finished" }>> {
  #client: MeridianClient;
  #options: AudioRenderOptions;
  #handlePromise: Promise<AudioRenderJobHandle> | null = null;
  #resultPromise:
    | Promise<Extract<AudioRenderEvent, { type: "render_finished" }>>
    | null = null;

  constructor(client: MeridianClient, options: AudioRenderOptions) {
    this.#client = client;
    this.#options = normalizeAudioRenderOptions(options);
  }

  start(): Promise<AudioRenderJobHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = this.#client.startAudioRender(this.#snapshot());
    }
    return this.#handlePromise;
  }

  then<
    TResult1 = Extract<AudioRenderEvent, { type: "render_finished" }>,
    TResult2 = never,
  >(
    onfulfilled?:
      | ((
        value: Extract<AudioRenderEvent, { type: "render_finished" }>,
      ) => TResult1 | PromiseLike<TResult1>)
      | null,
    onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
  ): Promise<TResult1 | TResult2> {
    if (!this.#resultPromise) {
      this.#resultPromise = this.start().then((handle) => handle.wait());
    }
    return this.#resultPromise.then(onfulfilled, onrejected);
  }

  catch<TResult = never>(
    onrejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
  ): Promise<Extract<AudioRenderEvent, { type: "render_finished" }> | TResult> {
    return Promise.resolve(this).catch(onrejected);
  }

  finally(
    onfinally?: (() => void) | null,
  ): Promise<Extract<AudioRenderEvent, { type: "render_finished" }>> {
    return Promise.resolve(this).finally(onfinally ?? undefined);
  }

  toJSON(): AudioRenderOptions {
    return this.#snapshot();
  }

  #snapshot(): AudioRenderOptions {
    return normalizeAudioRenderOptions(this.#options);
  }
}

export class VideoRenderTask
  implements
    PromiseLike<Extract<VideoRenderEvent, { type: "render_finished" }>> {
  #client: MeridianClient;
  #options: VideoRenderOptions;
  #handlePromise: Promise<VideoRenderJobHandle> | null = null;
  #resultPromise:
    | Promise<Extract<VideoRenderEvent, { type: "render_finished" }>>
    | null = null;

  constructor(client: MeridianClient, options: VideoRenderOptions) {
    this.#client = client;
    this.#options = normalizeVideoRenderOptions(options);
  }

  start(): Promise<VideoRenderJobHandle> {
    if (!this.#handlePromise) {
      this.#handlePromise = this.#client.startVideoRender(this.#snapshot());
    }
    return this.#handlePromise;
  }

  then<
    TResult1 = Extract<VideoRenderEvent, { type: "render_finished" }>,
    TResult2 = never,
  >(
    onfulfilled?:
      | ((
        value: Extract<VideoRenderEvent, { type: "render_finished" }>,
      ) => TResult1 | PromiseLike<TResult1>)
      | null,
    onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
  ): Promise<TResult1 | TResult2> {
    if (!this.#resultPromise) {
      this.#resultPromise = this.start().then((handle) => handle.wait());
    }
    return this.#resultPromise.then(onfulfilled, onrejected);
  }

  catch<TResult = never>(
    onrejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
  ): Promise<Extract<VideoRenderEvent, { type: "render_finished" }> | TResult> {
    return Promise.resolve(this).catch(onrejected);
  }

  finally(
    onfinally?: (() => void) | null,
  ): Promise<Extract<VideoRenderEvent, { type: "render_finished" }>> {
    return Promise.resolve(this).finally(onfinally ?? undefined);
  }

  toJSON(): VideoRenderOptions {
    return this.#snapshot();
  }

  #snapshot(): VideoRenderOptions {
    return normalizeVideoRenderOptions(this.#options);
  }
}
