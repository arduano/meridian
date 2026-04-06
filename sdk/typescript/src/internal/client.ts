import type { DeepPartial } from "../helpers.ts";
import { midiTools } from "../helpers.ts";
import {
  type AnalysisJobId,
  type AudioRenderEvent,
  type AudioRenderEventWrapper,
  type AudioRenderJobId,
  type AudioRenderStatus,
  type AudioRenderStatusEventWrapper,
  type ChangePpqTool,
  type ChannelRemapTool,
  type ControlChangeTool,
  type CoreEvent,
  type ErrorEvent,
  type ExtractTrackTool,
  type HumanizeTool,
  type JsonResponse,
  type KeyMapTool,
  type MeridianProtocolCommand,
  type MetaTextTool,
  type MidiAnalysisData,
  type MidiAnalysisJobEventWrapper,
  type MidiAnalysisJobStatus,
  type MidiAnalysisJobStatusEventWrapper,
  type MidiAnalysisKind,
  type MidiFilesMergeConfig,
  type MidiFilesMergedEvent,
  type MidiLoadedEvent,
  type MidiModifierTool,
  type MidiProcessEvent,
  type MidiProcessEventWrapper,
  type MidiProcessJobId,
  type MidiProcessStatus,
  type MidiProcessStatusEventWrapper,
  type NoteLengthTool,
  type ParsedMidiId,
  type PitchBendTool,
  type ProgramTool,
  PROTOCOL_VERSION,
  type ProtocolVideoRenderConfig,
  type QuantizeTool,
  type RangeSelectTool,
  type ResponseFor,
  type SdkAudioRenderConfig,
  type SysexTool,
  type TempoMapTool,
  type TempoPoint,
  type TimeWarpTool,
  type TrackRouteTool,
  type VelocityMapTool,
  type VideoRenderEvent,
  type VideoRenderEventWrapper,
  type VideoRenderJobId,
  type VideoRenderStatus,
  type VideoRenderStatusEventWrapper,
} from "../protocol.ts";
import type { MeridianRuntimeAdapter, MeridianSubprocess } from "./runtime.ts";

export class MeridianProtocolError extends Error {
  readonly code: ErrorEvent["code"];
  readonly events: CoreEvent[];

  constructor(event: ErrorEvent, events: CoreEvent[]) {
    super(event.message);
    this.name = "MeridianProtocolError";
    this.code = event.code;
    this.events = events;
  }
}

export class MeridianSubprocessError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MeridianSubprocessError";
  }
}

type PendingRequest = {
  resolve: (response: JsonResponse) => void;
  reject: (error: Error) => void;
};

export type MeridianEventListener = (event: CoreEvent) => void;

function isErrorEvent(event: CoreEvent): event is ErrorEvent {
  return event.type === "error";
}

function firstError(events: CoreEvent[]): ErrorEvent | null {
  return events.find(isErrorEvent) ?? null;
}

function assertNoError(events: CoreEvent[]): void {
  const event = firstError(events);
  if (event) {
    throw new MeridianProtocolError(event, events);
  }
}

function requireEvent<T extends CoreEvent["type"]>(
  events: CoreEvent[],
  type: T,
): Extract<CoreEvent, { type: T }> {
  assertNoError(events);
  const event = events.find(
    (candidate): candidate is Extract<CoreEvent, { type: T }> =>
      candidate.type === type,
  );
  if (!event) {
    throw new MeridianSubprocessError(
      `Expected event '${type}' but received ${
        events.map((item) => item.type).join(", ")
      }`,
    );
  }
  return event;
}

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

export interface AnalysisProgress {
  progress: number;
  status: string;
}

export class MidiAnalysisJobHandle {
  readonly jobId: AnalysisJobId;
  readonly parsedMidiId: ParsedMidiId;
  #protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #progressListeners = new Set<(progress: AnalysisProgress) => void>();
  #done: Promise<MidiAnalysisData>;
  #resolve!: (value: MidiAnalysisData) => void;
  #reject!: (error: Error) => void;
  #status: MidiAnalysisJobStatus;

  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: MidiAnalysisJobStatus,
  ) {
    this.#protocol = protocol;
    this.#status = initialStatus;
    this.jobId = initialStatus.job_id;
    this.parsedMidiId = initialStatus.parsed_midi_id;
    this.#done = new Promise<MidiAnalysisData>((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
    this.#unsubscribe = protocol.onEvent((event) => this.#handleEvent(event));
    this.#consumeStatus(initialStatus);
  }

  onProgress(listener: (progress: AnalysisProgress) => void): () => void {
    this.#progressListeners.add(listener);
    return () => {
      this.#progressListeners.delete(listener);
    };
  }

  async refreshStatus(): Promise<MidiAnalysisJobStatus> {
    const events = await this.#protocol.request({
      type: "get_midi_analysis_job_status",
      job_id: this.jobId,
    });
    const wrapper = requireEvent(events, "midi_analysis_job_status");
    this.#consumeStatus(wrapper.status);
    return this.#status;
  }

  wait(): Promise<MidiAnalysisData> {
    return this.#done;
  }

  #handleEvent(event: CoreEvent): void {
    if (event.type === "midi_analysis_job") {
      const wrapped = event as MidiAnalysisJobEventWrapper;
      if (
        wrapped.event.job_id === this.jobId && wrapped.event.type === "progress"
      ) {
        for (const listener of this.#progressListeners) {
          listener({
            progress: wrapped.event.progress,
            status: wrapped.event.status,
          });
        }
      }
      return;
    }

    if (event.type === "midi_analysis_job_status") {
      const wrapped = event as MidiAnalysisJobStatusEventWrapper;
      if (wrapped.status.job_id === this.jobId) {
        this.#consumeStatus(wrapped.status);
      }
    }
  }

  #consumeStatus(status: MidiAnalysisJobStatus): void {
    this.#status = status;
    if (status.state === "running") {
      for (const listener of this.#progressListeners) {
        listener({ progress: status.progress, status: status.status });
      }
      return;
    }
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    if (status.state === "finished") {
      this.#resolve(status.result);
      return;
    }
    this.#reject(new MeridianSubprocessError(status.message));
  }
}

export class MidiProcessJobHandle {
  readonly jobId: MidiProcessJobId;
  #protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #eventListeners = new Set<(event: MidiProcessEvent) => void>();
  #eventHistory: MidiProcessEvent[] = [];
  #settled = false;
  #done: Promise<Extract<MidiProcessEvent, { type: "process_finished" }>>;
  #resolve!: (
    value: Extract<MidiProcessEvent, { type: "process_finished" }>,
  ) => void;
  #reject!: (error: Error) => void;
  #status: MidiProcessStatus;

  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: MidiProcessStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create a process handle from idle status",
      );
    }
    this.#protocol = protocol;
    this.#status = initialStatus;
    this.jobId = initialStatus.job_id;
    this.#done = new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
    this.#unsubscribe = protocol.onEvent((event) => this.#handleEvent(event));
    for (const event of protocol.recentEvents()) {
      this.#handleEvent(event);
      if (this.#settled) {
        break;
      }
    }
  }

  onEvent(listener: (event: MidiProcessEvent) => void): () => void {
    this.#eventListeners.add(listener);
    for (const event of this.#eventHistory) {
      listener(event);
    }
    return () => {
      this.#eventListeners.delete(listener);
    };
  }

  async refreshStatus(): Promise<MidiProcessStatus> {
    const events = await this.#protocol.request({
      type: "get_midi_file_process_status",
    });
    const wrapper = requireEvent(events, "midi_process_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  async cancel(): Promise<MidiProcessStatus> {
    const events = await this.#protocol.request({
      type: "cancel_midi_file_process",
    });
    const wrapper = requireEvent(events, "midi_process_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  wait(): Promise<Extract<MidiProcessEvent, { type: "process_finished" }>> {
    return this.#done;
  }

  #handleEvent(event: CoreEvent): void {
    if (this.#settled) {
      return;
    }
    if (event.type === "midi_process_status") {
      this.#status = (event as MidiProcessStatusEventWrapper).status;
      return;
    }
    if (event.type !== "midi_process") {
      return;
    }
    const wrapped = event as MidiProcessEventWrapper;
    if (wrapped.event.job_id !== this.jobId) {
      return;
    }
    this.#eventHistory.push(wrapped.event);
    for (const listener of this.#eventListeners) {
      listener(wrapped.event);
    }
    switch (wrapped.event.type) {
      case "process_finished":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#resolve(wrapped.event);
        break;
      case "process_cancelled":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(
          new MeridianSubprocessError("MIDI processing job was cancelled"),
        );
        break;
      case "process_failed":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(new MeridianSubprocessError(wrapped.event.message));
        break;
      default:
        break;
    }
  }
}

export class AudioRenderJobHandle {
  readonly jobId: AudioRenderJobId;
  #protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #eventListeners = new Set<(event: AudioRenderEvent) => void>();
  #done: Promise<Extract<AudioRenderEvent, { type: "render_finished" }>>;
  #resolve!: (
    value: Extract<AudioRenderEvent, { type: "render_finished" }>,
  ) => void;
  #reject!: (error: Error) => void;
  #status: AudioRenderStatus;

  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: AudioRenderStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create an audio render handle from idle status",
      );
    }
    this.#protocol = protocol;
    this.#status = initialStatus;
    this.jobId = initialStatus.job_id;
    this.#done = new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
    this.#unsubscribe = protocol.onEvent((event) => this.#handleEvent(event));
  }

  onEvent(listener: (event: AudioRenderEvent) => void): () => void {
    this.#eventListeners.add(listener);
    return () => {
      this.#eventListeners.delete(listener);
    };
  }

  async refreshStatus(): Promise<AudioRenderStatus> {
    const events = await this.#protocol.request({
      type: "get_render_audio_status",
    });
    const wrapper = requireEvent(events, "audio_render_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  async cancel(): Promise<AudioRenderStatus> {
    const events = await this.#protocol.request({
      type: "cancel_render_audio",
    });
    const wrapper = requireEvent(events, "audio_render_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  wait(): Promise<Extract<AudioRenderEvent, { type: "render_finished" }>> {
    return this.#done;
  }

  #handleEvent(event: CoreEvent): void {
    if (event.type === "audio_render_status") {
      this.#status = (event as AudioRenderStatusEventWrapper).status;
      return;
    }
    if (event.type !== "audio_render") {
      return;
    }
    const wrapped = event as AudioRenderEventWrapper;
    if ("job_id" in wrapped.event && wrapped.event.job_id !== this.jobId) {
      return;
    }
    for (const listener of this.#eventListeners) {
      listener(wrapped.event);
    }
    switch (wrapped.event.type) {
      case "render_finished":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#resolve(wrapped.event);
        break;
      case "render_cancelled":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(
          new MeridianSubprocessError("Audio render job was cancelled"),
        );
        break;
      case "render_failed":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(new MeridianSubprocessError(wrapped.event.message));
        break;
      default:
        break;
    }
  }
}

export interface StartAnalysisForFileOptions {
  midiPath: string;
  kinds?: MidiAnalysisKind[];
  bucketCount?: number | null;
  onProgress?: (progress: AnalysisProgress) => void;
}

export interface MidiAnalysisOptions {
  file?: boolean;
  summary?: boolean;
  events?: boolean;
  notes?: boolean;
  tempo?: boolean;
  buckets?: number | boolean | null;
  onProgress?: (progress: AnalysisProgress) => void;
}

export interface AudioRenderOptions {
  midiPath: string;
  output: string;
  sampleRate?: number | null;
  channels?: number | null;
  useLimiter?: boolean | null;
  soundfonts?: string[];
  onEvent?: (event: AudioRenderEvent) => void;
}

export interface VideoRenderOptions {
  midiPath: string;
  output: string;
  fps: number;
  width: number;
  height: number;
  renderer?: ProtocolVideoRenderConfig["renderer"];
  scene?: ProtocolVideoRenderConfig["scene"];
  viewRange?: number | null;
  timeSpace?: ProtocolVideoRenderConfig["time_space"];
  firstKey?: number | null;
  lastKey?: number | null;
  ffmpegArgs?: string[];
  onEvent?: (event: VideoRenderEvent) => void;
}

export interface MidiToolTaskOptions {
  input: string;
  output: string;
  tool: MidiModifierTool;
  onEvent?: (event: MidiProcessEvent) => void;
}

export interface MidiModificationOptions {
  input: string;
  output: string;
  onEvent?: (event: MidiProcessEvent) => void;
}

export interface MidiMergeOptions {
  inputs: string[];
  output: string;
  config?: DeepPartial<MidiFilesMergeConfig>;
}

type ToolConfig<T extends MidiModifierTool> = Omit<T, "tool">;

function uniqueKinds(kinds: MidiAnalysisKind[]): MidiAnalysisKind[] {
  return [...new Set(kinds)];
}

function inferRendererFromScene(
  scene: ProtocolVideoRenderConfig["scene"] | undefined,
): ProtocolVideoRenderConfig["renderer"] {
  if (!scene) {
    return null;
  }
  if (scene.scene_type === "three_d") {
    return "piano_trail_classic";
  }
  return scene.notes.projector === "flat" ? "flat" : "pfa";
}

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

export class VideoRenderJobHandle {
  readonly jobId: VideoRenderJobId;
  #protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #eventListeners = new Set<(event: VideoRenderEvent) => void>();
  #done: Promise<Extract<VideoRenderEvent, { type: "render_finished" }>>;
  #resolve!: (
    value: Extract<VideoRenderEvent, { type: "render_finished" }>,
  ) => void;
  #reject!: (error: Error) => void;
  #status: VideoRenderStatus;

  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: VideoRenderStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create a video render handle from idle status",
      );
    }
    this.#protocol = protocol;
    this.#status = initialStatus;
    this.jobId = initialStatus.job_id;
    this.#done = new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
    });
    this.#unsubscribe = protocol.onEvent((event) => this.#handleEvent(event));
  }

  onEvent(listener: (event: VideoRenderEvent) => void): () => void {
    this.#eventListeners.add(listener);
    return () => {
      this.#eventListeners.delete(listener);
    };
  }

  async refreshStatus(): Promise<VideoRenderStatus> {
    const events = await this.#protocol.request({
      type: "get_render_video_status",
    });
    const wrapper = requireEvent(events, "video_render_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  async cancel(): Promise<VideoRenderStatus> {
    const events = await this.#protocol.request({
      type: "cancel_render_video",
    });
    const wrapper = requireEvent(events, "video_render_status");
    this.#status = wrapper.status;
    return this.#status;
  }

  wait(): Promise<Extract<VideoRenderEvent, { type: "render_finished" }>> {
    return this.#done;
  }

  #handleEvent(event: CoreEvent): void {
    if (event.type === "video_render_status") {
      this.#status = (event as VideoRenderStatusEventWrapper).status;
      return;
    }
    if (event.type !== "video_render") {
      return;
    }
    const wrapped = event as VideoRenderEventWrapper;
    if ("job_id" in wrapped.event && wrapped.event.job_id !== this.jobId) {
      return;
    }
    for (const listener of this.#eventListeners) {
      listener(wrapped.event);
    }
    switch (wrapped.event.type) {
      case "render_finished":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#resolve(wrapped.event);
        break;
      case "render_cancelled":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(
          new MeridianSubprocessError("Video render job was cancelled"),
        );
        break;
      case "render_failed":
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(new MeridianSubprocessError(wrapped.event.message));
        break;
      default:
        break;
    }
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
      ffmpegArgs: [...(this.#options.ffmpegArgs ?? [])],
      ...(this.#options.onEvent ? { onEvent: this.#options.onEvent } : {}),
    };
  }
}

export class MeridianClient {
  readonly protocol: MeridianProtocolClient;
  readonly resources = {
    loadParsedMidi: (path: string): Promise<ParsedMidiId> =>
      this.loadParsedMidi(path),
    loadAudioMidi: (path: string): Promise<MidiLoadedEvent> =>
      this.loadAudioMidi(path),
  };
  readonly audio = {
    render: (options: AudioRenderOptions): AudioRenderTask =>
      new AudioRenderTask(this, options),
  };
  readonly video = {
    render: (options: VideoRenderOptions): VideoRenderTask =>
      new VideoRenderTask(this, options),
  };
  readonly modification = {
    apply: (
      tool: MidiModifierTool,
      options: MidiModificationOptions,
    ): MidiProcessTask => this.midi({ ...options, tool }),
    rangeSelect: (
      options: MidiModificationOptions & Partial<ToolConfig<RangeSelectTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.rangeSelect(tool)),
    tempoMap: {
      flatten: (
        tempo: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.tempoMap.flatten(tempo) }),
      scaleBpm: (
        factor: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.tempoMap.scaleBpm(factor) }),
      replace: (
        points: TempoPoint[],
        destination: Parameters<typeof midiTools.tempoMap.replace>[1],
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({
          ...options,
          tool: midiTools.tempoMap.replace(points, destination),
        }),
    },
    timeWarp: (
      options: MidiModificationOptions & Partial<ToolConfig<TimeWarpTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.timeWarp(tool)),
    channelRemap: (
      options: MidiModificationOptions & Partial<ToolConfig<ChannelRemapTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.channelRemap(tool)),
    trackRoute: {
      collapseAll: (options: MidiModificationOptions): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.collapseAll() }),
      splitByChannel: (options: MidiModificationOptions): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.splitByChannel() }),
      map: (
        mappings: Parameters<typeof midiTools.trackRoute.map>[0],
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.map(mappings) }),
    },
    program: (
      options: MidiModificationOptions & Partial<ToolConfig<ProgramTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.program(tool)),
    controlChange: (
      options: MidiModificationOptions & Partial<ToolConfig<ControlChangeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.controlChange(tool)),
    pitchBend: (
      options: MidiModificationOptions & Partial<ToolConfig<PitchBendTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.pitchBend(tool)),
    velocityMap: {
      scale: (
        scale: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.scale(scale) }),
      gamma: (
        gamma: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.gamma(gamma) }),
      polyline: (
        points: Parameters<typeof midiTools.velocityMap.polyline>[0],
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.polyline(points) }),
    },
    changePpq: (
      ppq: number,
      options: MidiModificationOptions,
    ): MidiProcessTask =>
      this.midi({ ...options, tool: midiTools.changePpq({ ppq }) }),
    extractTrack: (
      trackIndex: number,
      options: MidiModificationOptions,
    ): MidiProcessTask =>
      this.midi({
        ...options,
        tool: midiTools.extractTrack({ track_index: trackIndex }),
      }),
    noteLength: (
      options: MidiModificationOptions & Partial<ToolConfig<NoteLengthTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.noteLength(tool)),
    quantize: (
      options: MidiModificationOptions & Partial<ToolConfig<QuantizeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.quantize(tool)),
    humanize: (
      options: MidiModificationOptions & Partial<ToolConfig<HumanizeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.humanize(tool)),
    keyMap: (
      options: MidiModificationOptions & Partial<ToolConfig<KeyMapTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.keyMap(tool)),
    metaText: (
      options: MidiModificationOptions & Partial<ToolConfig<MetaTextTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.metaText(tool)),
    sysex: (
      options: MidiModificationOptions & Partial<ToolConfig<SysexTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.sysex(tool)),
  };
  readonly merge = {
    midiFiles: (options: MidiMergeOptions): Promise<MidiFilesMergedEvent> =>
      this.mergeMidiFiles(options),
  };

  constructor(protocol: MeridianProtocolClient) {
    this.protocol = protocol;
  }

  analysis(
    midiPath: string,
    options: MidiAnalysisOptions = {},
  ): MidiAnalysisTask {
    return new MidiAnalysisTask(this, midiPath, options);
  }

  midi(options: MidiToolTaskOptions): MidiProcessTask {
    return new MidiProcessTask(this, options);
  }

  async close(): Promise<void> {
    await this.protocol.close();
  }

  #withTool<T extends MidiModifierTool>(
    options: MidiModificationOptions & Partial<ToolConfig<T>>,
    build: (config: Partial<ToolConfig<T>>) => T,
  ): MidiProcessTask {
    const { input, output, onEvent, ...toolConfig } = options;
    return this.midi({
      input,
      output,
      ...(onEvent ? { onEvent } : {}),
      tool: build(toolConfig as unknown as Partial<ToolConfig<T>>),
    });
  }

  async startAnalysis(
    options: StartAnalysisForFileOptions,
  ): Promise<MidiAnalysisJobHandle> {
    const parsedMidiId = await this.resources.loadParsedMidi(options.midiPath);
    const events = await this.protocol.request({
      type: "start_midi_analysis_job",
      parsed_midi_id: parsedMidiId,
      display_cache_id: null,
      kinds: options.kinds ?? [],
      bucket_count: options.bucketCount ?? null,
    });
    const wrapper = requireEvent(events, "midi_analysis_job_status");
    const handle = new MidiAnalysisJobHandle(this.protocol, wrapper.status);
    if (options.onProgress) {
      handle.onProgress(options.onProgress);
    }
    return handle;
  }

  async startMidiProcess(
    options: MidiToolTaskOptions,
  ): Promise<MidiProcessJobHandle> {
    const events = await this.protocol.request({
      type: "start_process_midi_file",
      input: options.input,
      output: options.output,
      config: { tool: structuredClone(options.tool) },
    });
    const wrapper = requireEvent(events, "midi_process_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "MIDI processing job did not enter a running state",
      );
    }
    const handle = new MidiProcessJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  async mergeMidiFiles(
    options: MidiMergeOptions,
  ): Promise<MidiFilesMergedEvent> {
    const config: MidiFilesMergeConfig = {
      mode: options.config?.mode ?? "append_tracks",
      normalize_metadata_track: options.config?.normalize_metadata_track ??
        false,
      ppq_override: options.config?.ppq_override ?? null,
    };
    const events = await this.protocol.request({
      type: "merge_midi_files",
      inputs: [...options.inputs],
      output: options.output,
      config,
    });
    return requireEvent(events, "midi_files_merged");
  }

  async startAudioRender(
    options: AudioRenderOptions,
  ): Promise<AudioRenderJobHandle> {
    await this.resources.loadAudioMidi(options.midiPath);

    const config: SdkAudioRenderConfig = {
      midi_path: options.midiPath,
      output: options.output,
      sample_rate: options.sampleRate ?? null,
      channels: options.channels ?? null,
      use_limiter: options.useLimiter ?? null,
      soundfonts: options.soundfonts ?? [],
    };
    const events = await this.protocol.request({
      type: "start_render_audio",
      config,
    });
    const wrapper = requireEvent(events, "audio_render_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "Audio render job did not enter a running state",
      );
    }
    const handle = new AudioRenderJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  async startVideoRender(
    options: VideoRenderOptions,
  ): Promise<VideoRenderJobHandle> {
    const renderer = options.renderer ?? inferRendererFromScene(options.scene);
    if (!renderer && !options.scene) {
      throw new MeridianSubprocessError(
        "Video render requires either a renderer or a scene config",
      );
    }
    const config: ProtocolVideoRenderConfig = {
      midi_path: options.midiPath,
      output: options.output,
      fps: options.fps,
      width: options.width,
      height: options.height,
      renderer,
      scene: options.scene ? structuredClone(options.scene) : null,
      view_range: options.viewRange ?? null,
      time_space: options.timeSpace ?? null,
      first_key: options.firstKey ?? null,
      last_key: options.lastKey ?? null,
      ffmpeg_args: options.ffmpegArgs ?? [],
    };
    const events = await this.protocol.request({
      type: "start_render_video",
      config,
    });
    const wrapper = requireEvent(events, "video_render_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "Video render job did not enter a running state",
      );
    }
    const handle = new VideoRenderJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  private async loadParsedMidi(path: string): Promise<ParsedMidiId> {
    const events = await this.protocol.request({
      type: "load_parsed_midi",
      path,
    });
    const event = requireEvent(events, "parsed_midi_loaded");
    return event.parsed_midi_id;
  }

  private async loadAudioMidi(path: string): Promise<MidiLoadedEvent> {
    const events = await this.protocol.request({
      type: "load_audio_midi",
      path,
    });
    return requireEvent(events, "midi_loaded");
  }
}
