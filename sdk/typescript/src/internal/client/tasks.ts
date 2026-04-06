import type {
  AudioRenderEvent,
  MidiAnalysisData,
  MidiAnalysisKind,
  MidiFilesMergeConfig,
  MidiFilesMergedEvent,
  MidiModifierTool,
  MidiProcessEvent,
  ParsedMidiId,
  ProtocolVideoRenderConfig,
  SdkAudioRenderConfig,
  TempoPoint,
  TimeWarpTool,
  TrackRouteTool,
  VideoRenderEvent,
} from "../../protocol.ts";
import { midiTools } from "../../helpers.ts";
import type { MeridianClient } from "./meridian_client.ts";
import {
  inferRendererFromScene,
  type ToolConfig,
  uniqueKinds,
} from "./internal.ts";
import {
  AudioRenderJobHandle,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
  VideoRenderJobHandle,
} from "./handles.ts";
import type {
  AudioRenderOptions,
  MidiAnalysisOptions,
  MidiMergeOptions,
  MidiModificationOptions,
  MidiToolTaskOptions,
  StartAnalysisForFileOptions,
  VideoRenderOptions,
} from "./options.ts";
import { MeridianSubprocessError, requireEvent } from "./internal.ts";

function analysisOptionsToSpec(
  midiPath: string,
  options: MidiAnalysisOptions = {},
): StartAnalysisForFileOptions {
  const kinds: MidiAnalysisKind[] = [];
  if (options.file) kinds.push("file");
  if (options.summary) kinds.push("summary");
  if (options.events) kinds.push("events");
  if (options.notes) kinds.push("notes");
  if (options.tempo) kinds.push("tempo");

  let bucketCount: number | null = null;
  if (options.buckets === true) {
    kinds.push("buckets");
    bucketCount = 256;
  } else if (typeof options.buckets === "number") {
    kinds.push("buckets");
    bucketCount = options.buckets;
  }

  return {
    midiPath,
    kinds: uniqueKinds(kinds),
    bucketCount,
    ...(options.onProgress ? { onProgress: options.onProgress } : {}),
  };
}

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
    this.#spec = analysisOptionsToSpec(midiPath, options);
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
      kinds: [...(this.#spec.kinds ?? [])],
      bucketCount: this.#spec.bucketCount ?? null,
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
    this.#options = {
      input: options.input,
      output: options.output,
      tool: structuredClone(options.tool),
      ...(options.onEvent ? { onEvent: options.onEvent } : {}),
    };
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
    return {
      input: this.#options.input,
      output: this.#options.output,
      tool: structuredClone(this.#options.tool),
      ...(this.#options.onEvent ? { onEvent: this.#options.onEvent } : {}),
    };
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
    this.#options = {
      midiPath: options.midiPath,
      output: options.output,
      sampleRate: options.sampleRate ?? null,
      channels: options.channels ?? null,
      useLimiter: options.useLimiter ?? null,
      soundfonts: options.soundfonts ? [...options.soundfonts] : [],
      ...(options.onEvent ? { onEvent: options.onEvent } : {}),
    };
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
    return {
      midiPath: this.#options.midiPath,
      output: this.#options.output,
      sampleRate: this.#options.sampleRate ?? null,
      channels: this.#options.channels ?? null,
      useLimiter: this.#options.useLimiter ?? null,
      soundfonts: [...(this.#options.soundfonts ?? [])],
      ...(this.#options.onEvent ? { onEvent: this.#options.onEvent } : {}),
    };
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
    const renderer = options.renderer ?? inferRendererFromScene(options.scene);
    this.#options = {
      midiPath: options.midiPath,
      output: options.output,
      fps: options.fps,
      width: options.width,
      height: options.height,
      renderer,
      scene: options.scene ? structuredClone(options.scene) : null,
      viewRange: options.viewRange ?? null,
      timeSpace: options.timeSpace ?? null,
      firstKey: options.firstKey ?? null,
      lastKey: options.lastKey ?? null,
      rgbMode: options.rgbMode ?? "premultiplied",
      exportAlphaMask: options.exportAlphaMask ?? false,
      ffmpegArgs: options.ffmpegArgs ? [...options.ffmpegArgs] : [],
      ...(options.onEvent ? { onEvent: options.onEvent } : {}),
    };
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
    return {
      midiPath: this.#options.midiPath,
      output: this.#options.output,
      fps: this.#options.fps,
      width: this.#options.width,
      height: this.#options.height,
      renderer: this.#options.renderer ?? null,
      scene: this.#options.scene ? structuredClone(this.#options.scene) : null,
      viewRange: this.#options.viewRange ?? null,
      timeSpace: this.#options.timeSpace ?? null,
      firstKey: this.#options.firstKey ?? null,
      lastKey: this.#options.lastKey ?? null,
      rgbMode: this.#options.rgbMode ?? "premultiplied",
      exportAlphaMask: this.#options.exportAlphaMask ?? false,
      ffmpegArgs: [...(this.#options.ffmpegArgs ?? [])],
      ...(this.#options.onEvent ? { onEvent: this.#options.onEvent } : {}),
    };
  }
}
