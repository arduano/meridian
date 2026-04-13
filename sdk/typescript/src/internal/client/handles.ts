import type {
  AnalysisJobId,
  AudioRenderEvent,
  AudioRenderEventWrapper,
  AudioRenderStatus,
  AudioRenderStatusEventWrapper,
  CoreEvent,
  MidiAnalysisData,
  MidiAnalysisJobEventWrapper,
  MidiAnalysisJobStatus,
  MidiAnalysisJobStatusEventWrapper,
  MidiProcessEvent,
  MidiProcessEventWrapper,
  MidiProcessStatus,
  MidiProcessStatusEventWrapper,
  ParsedMidiId,
  VideoRenderEvent,
  VideoRenderEventWrapper,
  VideoRenderStatus,
  VideoRenderStatusEventWrapper,
} from "../../protocol.ts";
import type { MeridianProtocolClient } from "./protocol_client.ts";
import { MeridianSubprocessError, requireEvent } from "./internal.ts";
import { ReplayableJobHandle } from "./replayable_job_handle.ts";

export interface AnalysisProgress {
  progress: number;
  status: string;
}

/** Live analysis-job handle with replay-safe progress and final result waiting. */
export class MidiAnalysisJobHandle extends ReplayableJobHandle<
  MidiAnalysisJobStatus,
  Extract<MidiAnalysisJobEventWrapper["event"], { job_id: AnalysisJobId }>,
  Extract<MidiAnalysisJobEventWrapper["event"], { type: "finished" }>,
  MidiAnalysisData
> {
  readonly parsedMidiId: ParsedMidiId;
  #progressListeners = new Set<(progress: AnalysisProgress) => void>();
  #progressHistory: AnalysisProgress[] = [];
  #lastProgressKey: string | null = null;

  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: MidiAnalysisJobStatus,
  ) {
    super(protocol, initialStatus);
    this.parsedMidiId = initialStatus.parsed_midi_id;
    this.#consumeStatus(initialStatus);
    if (!this.isSettled()) {
      this.startTracking();
    }
  }

  onProgress(listener: (progress: AnalysisProgress) => void): () => void {
    this.#progressListeners.add(listener);
    for (const progress of this.#progressHistory) {
      listener(progress);
    }
    return () => {
      this.#progressListeners.delete(listener);
    };
  }

  override wait(): Promise<MidiAnalysisData> {
    return super.wait();
  }

  protected override resolveFinished(
    event: Extract<
      MidiAnalysisJobEventWrapper["event"],
      { type: "finished" }
    >,
  ): MidiAnalysisData {
    return event.result;
  }

  protected override handleCoreEvent(event: CoreEvent): void {
    if (this.isSettled()) {
      return;
    }
    if (event.type === "midi_analysis_job") {
      const wrapped = event as MidiAnalysisJobEventWrapper;
      if (wrapped.event.job_id !== this.jobId) {
        return;
      }
      switch (wrapped.event.type) {
        case "started":
          this.#emitProgress({ progress: 0, status: "Started" });
          break;
        case "progress":
          this.#emitProgress({
            progress: wrapped.event.progress,
            status: wrapped.event.status,
          });
          break;
        case "finished":
          this.finish(wrapped.event);
          break;
        case "failed":
          this.fail(wrapped.event.message);
          break;
        default:
          break;
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
    this.updateStatus(status);
    if (status.state === "running") {
      this.#emitProgress({ progress: status.progress, status: status.status });
      return;
    }
    if (status.state === "finished") {
      this.finish({
        type: "finished",
        job_id: status.job_id,
        result: status.result,
      });
      return;
    }
    this.fail(status.message);
  }

  #emitProgress(progress: AnalysisProgress): void {
    const key = `${progress.progress}\0${progress.status}`;
    if (this.#lastProgressKey === key) {
      return;
    }
    this.#lastProgressKey = key;
    this.#progressHistory.push(progress);
    for (const listener of this.#progressListeners) {
      listener(progress);
    }
  }
}

/** Live MIDI-processing job handle with refresh/cancel support. */
export class MidiProcessJobHandle extends ReplayableJobHandle<
  MidiProcessStatus,
  MidiProcessEvent,
  Extract<MidiProcessEvent, { type: "process_finished" }>
> {
  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: MidiProcessStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create a process handle from idle status",
      );
    }
    super(protocol, initialStatus);
    this.startTracking();
  }

  protected override resolveFinished(
    event: Extract<MidiProcessEvent, { type: "process_finished" }>,
  ): Extract<MidiProcessEvent, { type: "process_finished" }> {
    return event;
  }

  async refreshStatus(): Promise<MidiProcessStatus> {
    const events = await this.protocol.request({
      type: "get_midi_file_process_status",
    });
    const wrapper = requireEvent(events, "midi_process_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  async cancel(): Promise<MidiProcessStatus> {
    const events = await this.protocol.request({
      type: "cancel_midi_file_process",
    });
    const wrapper = requireEvent(events, "midi_process_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  protected handleCoreEvent(event: CoreEvent): void {
    if (this.isSettled()) {
      return;
    }
    if (event.type === "midi_process_status") {
      this.updateStatus((event as MidiProcessStatusEventWrapper).status);
      return;
    }
    if (event.type !== "midi_process") {
      return;
    }
    const wrapped = event as MidiProcessEventWrapper;
    if (wrapped.event.job_id !== this.jobId) {
      return;
    }
    this.emitEvent(wrapped.event);
    switch (wrapped.event.type) {
      case "process_finished":
        this.finish(wrapped.event);
        break;
      case "process_cancelled":
        this.fail("MIDI processing job was cancelled");
        break;
      case "process_failed":
        this.fail(wrapped.event.message);
        break;
      default:
        break;
    }
  }
}

/** Live audio-render job handle with refresh/cancel support. */
export class AudioRenderJobHandle extends ReplayableJobHandle<
  AudioRenderStatus,
  AudioRenderEvent,
  Extract<AudioRenderEvent, { type: "render_finished" }>
> {
  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: AudioRenderStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create an audio render handle from idle status",
      );
    }
    super(protocol, initialStatus);
    this.startTracking();
  }

  protected override resolveFinished(
    event: Extract<AudioRenderEvent, { type: "render_finished" }>,
  ): Extract<AudioRenderEvent, { type: "render_finished" }> {
    return event;
  }

  async refreshStatus(): Promise<AudioRenderStatus> {
    const events = await this.protocol.request({
      type: "get_render_audio_status",
    });
    const wrapper = requireEvent(events, "audio_render_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  async cancel(): Promise<AudioRenderStatus> {
    const events = await this.protocol.request({
      type: "cancel_render_audio",
    });
    const wrapper = requireEvent(events, "audio_render_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  protected handleCoreEvent(event: CoreEvent): void {
    if (this.isSettled()) {
      return;
    }
    if (event.type === "audio_render_status") {
      this.updateStatus((event as AudioRenderStatusEventWrapper).status);
      return;
    }
    if (event.type !== "audio_render") {
      return;
    }
    const wrapped = event as AudioRenderEventWrapper;
    if ("job_id" in wrapped.event && wrapped.event.job_id !== this.jobId) {
      return;
    }
    this.emitEvent(wrapped.event);
    switch (wrapped.event.type) {
      case "render_finished":
        this.finish(wrapped.event);
        break;
      case "render_cancelled":
        this.fail("Audio render job was cancelled");
        break;
      case "render_failed":
        this.fail(wrapped.event.message);
        break;
      default:
        break;
    }
  }
}

export class VideoRenderJobHandle extends ReplayableJobHandle<
  VideoRenderStatus,
  VideoRenderEvent,
  Extract<VideoRenderEvent, { type: "render_finished" }>
> {
  constructor(
    protocol: MeridianProtocolClient,
    initialStatus: VideoRenderStatus,
  ) {
    if (initialStatus.state === "idle") {
      throw new MeridianSubprocessError(
        "Cannot create a video render handle from idle status",
      );
    }
    super(protocol, initialStatus);
    this.startTracking();
  }

  protected override resolveFinished(
    event: Extract<VideoRenderEvent, { type: "render_finished" }>,
  ): Extract<VideoRenderEvent, { type: "render_finished" }> {
    return event;
  }

  async refreshStatus(): Promise<VideoRenderStatus> {
    const events = await this.protocol.request({
      type: "get_render_video_status",
    });
    const wrapper = requireEvent(events, "video_render_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  async cancel(): Promise<VideoRenderStatus> {
    const events = await this.protocol.request({
      type: "cancel_render_video",
    });
    const wrapper = requireEvent(events, "video_render_status");
    this.updateStatus(wrapper.status);
    return this.status;
  }

  protected handleCoreEvent(event: CoreEvent): void {
    if (this.isSettled()) {
      return;
    }
    if (event.type === "video_render_status") {
      this.updateStatus((event as VideoRenderStatusEventWrapper).status);
      return;
    }
    if (event.type !== "video_render") {
      return;
    }
    const wrapped = event as VideoRenderEventWrapper;
    if ("job_id" in wrapped.event && wrapped.event.job_id !== this.jobId) {
      return;
    }
    this.emitEvent(wrapped.event);
    switch (wrapped.event.type) {
      case "render_finished":
        this.finish(wrapped.event);
        break;
      case "render_cancelled":
        this.fail("Video render job was cancelled");
        break;
      case "render_failed":
        this.fail(wrapped.event.message);
        break;
      default:
        break;
    }
  }
}
