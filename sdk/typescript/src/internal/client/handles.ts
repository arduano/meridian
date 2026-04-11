import type {
  AnalysisJobId,
  AudioRenderEvent,
  AudioRenderEventWrapper,
  AudioRenderJobId,
  AudioRenderStatus,
  AudioRenderStatusEventWrapper,
  CoreEvent,
  MidiAnalysisData,
  MidiAnalysisJobEventWrapper,
  MidiAnalysisJobStatus,
  MidiAnalysisJobStatusEventWrapper,
  MidiProcessEvent,
  MidiProcessEventWrapper,
  MidiProcessJobId,
  MidiProcessStatus,
  MidiProcessStatusEventWrapper,
  ParsedMidiId,
  VideoRenderEvent,
  VideoRenderEventWrapper,
  VideoRenderJobId,
  VideoRenderStatus,
  VideoRenderStatusEventWrapper,
} from "../../protocol.ts";
import type { MeridianProtocolClient } from "./protocol_client.ts";
import { MeridianSubprocessError, requireEvent } from "./internal.ts";

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
  #eventHistory: AudioRenderEvent[] = [];
  #settled = false;
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
    for (const event of protocol.recentEvents()) {
      this.#handleEvent(event);
      if (this.#settled) {
        break;
      }
    }
  }

  onEvent(listener: (event: AudioRenderEvent) => void): () => void {
    this.#eventListeners.add(listener);
    for (const event of this.#eventHistory) {
      listener(event);
    }
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
    if (this.#settled) {
      return;
    }
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
    this.#eventHistory.push(wrapped.event);
    for (const listener of this.#eventListeners) {
      listener(wrapped.event);
    }
    switch (wrapped.event.type) {
      case "render_finished":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#resolve(wrapped.event);
        break;
      case "render_cancelled":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(
          new MeridianSubprocessError("Audio render job was cancelled"),
        );
        break;
      case "render_failed":
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

export class VideoRenderJobHandle {
  readonly jobId: VideoRenderJobId;
  #protocol: MeridianProtocolClient;
  #unsubscribe: (() => void) | null = null;
  #eventListeners = new Set<(event: VideoRenderEvent) => void>();
  #eventHistory: VideoRenderEvent[] = [];
  #settled = false;
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
    for (const event of protocol.recentEvents()) {
      this.#handleEvent(event);
      if (this.#settled) {
        break;
      }
    }
  }

  onEvent(listener: (event: VideoRenderEvent) => void): () => void {
    this.#eventListeners.add(listener);
    for (const event of this.#eventHistory) {
      listener(event);
    }
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
    if (this.#settled) {
      return;
    }
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
    this.#eventHistory.push(wrapped.event);
    for (const listener of this.#eventListeners) {
      listener(wrapped.event);
    }
    switch (wrapped.event.type) {
      case "render_finished":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#resolve(wrapped.event);
        break;
      case "render_cancelled":
        this.#settled = true;
        this.#unsubscribe?.();
        this.#unsubscribe = null;
        this.#reject(
          new MeridianSubprocessError("Video render job was cancelled"),
        );
        break;
      case "render_failed":
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
