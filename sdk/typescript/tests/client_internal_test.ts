import {
  AudioRenderJobHandle,
  MeridianClient,
  MidiAnalysisTask,
  VideoRenderJobHandle,
} from "../src/internal/client.ts";
import type { MeridianProtocolClient } from "../src/internal/client.ts";
import type { CoreEvent } from "../src/protocol.ts";

class FakeProtocolClient {
  readonly requests: unknown[] = [];
  #listeners = new Set<(event: CoreEvent) => void>();
  #recentEvents: CoreEvent[];
  #requestHandler: (command: unknown) => CoreEvent[];

  constructor(
    options: {
      recentEvents?: CoreEvent[];
      requestHandler?: (command: unknown) => CoreEvent[];
    } = {},
  ) {
    this.#recentEvents = [...(options.recentEvents ?? [])];
    this.#requestHandler = options.requestHandler ?? (() => []);
  }

  onEvent(listener: (event: CoreEvent) => void): () => void {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  recentEvents(): readonly CoreEvent[] {
    return this.#recentEvents;
  }

  async request(command: unknown): Promise<CoreEvent[]> {
    this.requests.push(command);
    return this.#requestHandler(command);
  }

  async close(): Promise<void> {
    return Promise.resolve();
  }

  emit(event: CoreEvent): void {
    this.#recentEvents.push(event);
    for (const listener of this.#listeners) {
      listener(event);
    }
  }
}

const DEFAULT_ANALYSIS_KINDS = [
  "file",
  "summary",
  "events",
  "notes",
  "tempo",
];

Deno.test("analysis task defaults match CLI defaults without buckets", () => {
  const task = new MidiAnalysisTask(
    {} as unknown as MeridianClient,
    "song.mid",
  );

  const spec = task.toJSON();
  if (JSON.stringify(spec.kinds) !== JSON.stringify(DEFAULT_ANALYSIS_KINDS)) {
    throw new Error(`Unexpected default kinds: ${JSON.stringify(spec.kinds)}`);
  }
  if (spec.bucketCount !== null) {
    throw new Error(`Expected null bucketCount, got ${String(spec.bucketCount)}`);
  }
});

Deno.test("analysis task adds buckets on top of CLI defaults", () => {
  const task = new MidiAnalysisTask(
    {} as unknown as MeridianClient,
    "song.mid",
    { buckets: 8 },
  );

  const spec = task.toJSON();
  const expectedKinds = [...DEFAULT_ANALYSIS_KINDS, "buckets"];
  if (JSON.stringify(spec.kinds) !== JSON.stringify(expectedKinds)) {
    throw new Error(`Unexpected bucket kinds: ${JSON.stringify(spec.kinds)}`);
  }
  if (spec.bucketCount !== 8) {
    throw new Error(`Expected bucketCount 8, got ${String(spec.bucketCount)}`);
  }
});

Deno.test("analysis task still supports explicit subset selection", () => {
  const task = new MidiAnalysisTask(
    {} as unknown as MeridianClient,
    "song.mid",
    { summary: true, notes: true },
  );

  const spec = task.toJSON();
  const expectedKinds = ["summary", "notes"];
  if (JSON.stringify(spec.kinds) !== JSON.stringify(expectedKinds)) {
    throw new Error(`Unexpected explicit kinds: ${JSON.stringify(spec.kinds)}`);
  }
});

Deno.test("audio render handle replays a recent finished event", async () => {
  const finishedEvent: CoreEvent = {
    type: "audio_render",
    event: {
      type: "render_finished",
      job_id: 17,
      output: "out.wav",
      frames_written: 44_100,
      rendered_seconds: 1,
    },
  };
  const protocol = new FakeProtocolClient({ recentEvents: [finishedEvent] });
  const handle = new AudioRenderJobHandle(
    protocol as unknown as MeridianProtocolClient,
    {
      state: "running",
      job_id: 17,
      output: "out.wav",
      total_events: 10,
      event_index: 9,
      time_seconds: 1,
      rendered_seconds: 1,
      frames_written: 44_100,
    },
  );

  const result = await handle.wait();
  const seen: string[] = [];
  handle.onEvent((event) => seen.push(event.type));

  if (result.output !== "out.wav") {
    throw new Error(`Unexpected audio output: ${result.output}`);
  }
  if (JSON.stringify(seen) !== JSON.stringify(["render_finished"])) {
    throw new Error(`Unexpected audio replayed events: ${JSON.stringify(seen)}`);
  }
});

Deno.test("video render handle replays a recent finished event", async () => {
  const finishedEvent: CoreEvent = {
    type: "video_render",
    event: {
      type: "render_finished",
      job_id: 29,
      total_frames: 12,
      elapsed_seconds: 2,
      average_fps: 6,
      output: "out.mp4",
      container: "mp4",
      exports: {
        alpha_mask: null,
      },
    },
  };
  const protocol = new FakeProtocolClient({ recentEvents: [finishedEvent] });
  const handle = new VideoRenderJobHandle(
    protocol as unknown as MeridianProtocolClient,
    {
      state: "running",
      job_id: 29,
      output: "out.mp4",
      container: "mp4",
      fps: 6,
      width: 320,
      height: 180,
      total_frames: 12,
      frame_index: 11,
      current_time: 2,
      elapsed_seconds: 2,
    },
  );

  const result = await handle.wait();
  const seen: string[] = [];
  handle.onEvent((event) => seen.push(event.type));

  if (result.output !== "out.mp4") {
    throw new Error(`Unexpected video output: ${result.output}`);
  }
  if (JSON.stringify(seen) !== JSON.stringify(["render_finished"])) {
    throw new Error(`Unexpected video replayed events: ${JSON.stringify(seen)}`);
  }
});

Deno.test("audio render start forwards encoded-output options", async () => {
  const protocol = new FakeProtocolClient({
    requestHandler: (command) => {
      const typed = command as { type: string; path?: string; config?: unknown };
      switch (typed.type) {
        case "load_audio_midi":
          return [{ type: "midi_loaded", path: typed.path ?? "song.mid" }];
        case "start_render_audio":
          return [{
            type: "audio_render_status",
            status: {
              state: "running",
              job_id: 41,
              output: "out.mp3",
              total_events: 10,
              event_index: 0,
              time_seconds: 0,
              rendered_seconds: 0,
              frames_written: 0,
            },
          }];
        default:
          throw new Error(`Unexpected command: ${typed.type}`);
      }
    },
  });
  const client = new MeridianClient(
    protocol as unknown as MeridianProtocolClient,
  );

  await client.startAudioRender({
    midiPath: "song.mid",
    output: "out.mp3",
    sampleRate: 22_050,
    channels: 2,
    useLimiter: true,
    format: "mp3",
    ffmpegArgs: ["-b:a", "96k"],
    soundfonts: ["piano.sf2"],
  });

  if (protocol.requests.length !== 2) {
    throw new Error(`Expected 2 requests, got ${protocol.requests.length}`);
  }
  const start = protocol.requests[1] as {
    type: string;
    config: {
      midi_path: string;
      output: string;
      sample_rate: number | null;
      channels: number | null;
      use_limiter: boolean | null;
      format: string;
      ffmpeg_args: string[];
      soundfonts: string[];
    };
  };
  if (start.type !== "start_render_audio") {
    throw new Error(`Unexpected start command: ${start.type}`);
  }
  if (start.config.format !== "mp3") {
    throw new Error(`Expected mp3 format, got ${start.config.format}`);
  }
  if (JSON.stringify(start.config.ffmpeg_args) !== JSON.stringify(["-b:a", "96k"])) {
    throw new Error(
      `Unexpected ffmpeg args: ${JSON.stringify(start.config.ffmpeg_args)}`,
    );
  }
  if (JSON.stringify(start.config.soundfonts) !== JSON.stringify(["piano.sf2"])) {
    throw new Error(
      `Unexpected soundfonts: ${JSON.stringify(start.config.soundfonts)}`,
    );
  }
});
