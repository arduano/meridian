import {
  AudioRenderJobHandle,
  MeridianClient,
  MidiAnalysisTask,
  VideoRenderTask,
  VideoRenderJobHandle,
} from "../src/internal/client.ts";
import type { MeridianProtocolClient } from "../src/internal/client.ts";
import type { CoreEvent, SceneConfig } from "../src/protocol.ts";

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

const DEFAULT_TWO_D_SCENE: SceneConfig = {
  scene_type: "two_d",
  background: {
    source: "none",
  },
  keyboard_height: {
    mode: "screen_percent",
    height: 0.15,
  },
  notes: {
    projector: "pfa",
    same_width_notes: false,
    border_width: 0.04,
    palette: {
      source: "default_track_colors",
    },
  },
  keyboard: {
    projector: "pfa",
    same_width_notes: false,
    middle_c: false,
    top_bar_color: "#000000",
  },
};

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

Deno.test("video render task defaults renderer to pfa without a scene", () => {
  const task = new VideoRenderTask(
    {} as unknown as MeridianClient,
    {
      midiPath: "song.mid",
      output: "out.mp4",
      fps: 30,
      width: 160,
      height: 90,
    },
  );

  const spec = task.toJSON();
  if (spec.renderer !== "pfa") {
    throw new Error(`Expected default renderer pfa, got ${String(spec.renderer)}`);
  }
  if (spec.scene !== null) {
    throw new Error(`Expected null scene, got ${JSON.stringify(spec.scene)}`);
  }
});

Deno.test("video render task infers renderer from the scene", () => {
  const task = new VideoRenderTask(
    {} as unknown as MeridianClient,
    {
      midiPath: "song.mid",
      output: "out.mp4",
      fps: 30,
      width: 160,
      height: 90,
      scene: DEFAULT_TWO_D_SCENE,
    },
  );

  const spec = task.toJSON();
  if (spec.renderer !== "pfa") {
    throw new Error(`Expected inferred renderer pfa, got ${String(spec.renderer)}`);
  }
  if (JSON.stringify(spec.scene) !== JSON.stringify(DEFAULT_TWO_D_SCENE)) {
    throw new Error(`Unexpected scene snapshot: ${JSON.stringify(spec.scene)}`);
  }
});

Deno.test("video render task rejects mismatched renderer and scene", () => {
  let error: unknown = null;
  try {
    new VideoRenderTask(
      {} as unknown as MeridianClient,
      {
        midiPath: "song.mid",
        output: "out.mp4",
        fps: 30,
        width: 160,
        height: 90,
        renderer: "flat",
        scene: DEFAULT_TWO_D_SCENE,
      },
    );
  } catch (thrown) {
    error = thrown;
  }

  if (!(error instanceof Error)) {
    throw new Error("Expected mismatched video render options to throw");
  }
  if (!error.message.includes("renderer")) {
    throw new Error(`Unexpected error message: ${error.message}`);
  }
});

Deno.test("video render task preserves muxed audio options", () => {
  const soundfonts = ["piano.sf2"];
  const ffmpegArgs = ["-b:a", "96k"];
  const task = new VideoRenderTask(
    {} as unknown as MeridianClient,
    {
      midiPath: "song.mid",
      output: "out.mkv",
      fps: 30,
      width: 160,
      height: 90,
      audio: {
        sampleRate: 22_050,
        channels: 2,
        useLimiter: true,
        soundfonts,
        ffmpegArgs,
      },
    },
  );

  soundfonts[0] = "mutated.sf2";
  ffmpegArgs[0] = "-c:a";

  const spec = task.toJSON();
  if (spec.audio?.sampleRate !== 22_050) {
    throw new Error(`Expected sampleRate 22050, got ${String(spec.audio?.sampleRate)}`);
  }
  if (spec.audio?.channels !== 2) {
    throw new Error(`Expected channels 2, got ${String(spec.audio?.channels)}`);
  }
  if (spec.audio?.useLimiter !== true) {
    throw new Error(`Expected useLimiter true, got ${String(spec.audio?.useLimiter)}`);
  }
  if (JSON.stringify(spec.audio?.soundfonts) !== JSON.stringify(["piano.sf2"])) {
    throw new Error(`Unexpected soundfonts snapshot: ${JSON.stringify(spec.audio?.soundfonts)}`);
  }
  if (JSON.stringify(spec.audio?.ffmpegArgs) !== JSON.stringify(["-b:a", "96k"])) {
    throw new Error(`Unexpected ffmpeg args snapshot: ${JSON.stringify(spec.audio?.ffmpegArgs)}`);
  }
});

Deno.test("video render request forwards muxed audio options", async () => {
  const protocol = new FakeProtocolClient({
    requestHandler: (command) => {
      const typed = command as { type: string };
      if (typed.type !== "start_render_video") {
        throw new Error(`Unexpected command: ${typed.type}`);
      }
      return [
        {
          type: "video_render_status",
          status: {
            state: "running",
            job_id: 29,
            output: "out.mkv",
            container: "mkv",
            fps: 30,
            width: 160,
            height: 90,
            total_frames: 1,
            frame_index: 0,
            current_time: 0,
            elapsed_seconds: 0,
          },
        } as CoreEvent,
      ];
    },
  });
  const client = new MeridianClient(
    protocol as unknown as MeridianProtocolClient,
  );

  const soundfonts = ["piano.sf2"];
  const ffmpegArgs = ["-b:a", "96k"];
  await client.startVideoRender({
    midiPath: "song.mid",
    output: "out.mkv",
    fps: 30,
    width: 160,
    height: 90,
    audio: {
      sampleRate: 22_050,
      channels: 2,
      useLimiter: true,
      soundfonts,
      ffmpegArgs,
    },
  });

  if (protocol.requests.length !== 1) {
    throw new Error(`Expected 1 request, got ${protocol.requests.length}`);
  }
  const start = protocol.requests[0] as {
    type: string;
    config: {
      audio: {
        sample_rate: number | null;
        channels: number | null;
        use_limiter: boolean | null;
        soundfonts: string[];
        ffmpeg_args: string[];
      } | null;
    };
  };
  if (start.type !== "start_render_video") {
    throw new Error(`Unexpected start command: ${start.type}`);
  }
  if (start.config.audio === null) {
    throw new Error("Expected muxed audio config to be serialized");
  }
  if (start.config.audio.sample_rate !== 22_050) {
    throw new Error(`Expected sample_rate 22050, got ${String(start.config.audio.sample_rate)}`);
  }
  if (start.config.audio.channels !== 2) {
    throw new Error(`Expected channels 2, got ${String(start.config.audio.channels)}`);
  }
  if (start.config.audio.use_limiter !== true) {
    throw new Error(`Expected use_limiter true, got ${String(start.config.audio.use_limiter)}`);
  }
  if (JSON.stringify(start.config.audio.soundfonts) !== JSON.stringify(["piano.sf2"])) {
    throw new Error(`Unexpected soundfonts: ${JSON.stringify(start.config.audio.soundfonts)}`);
  }
  if (JSON.stringify(start.config.audio.ffmpeg_args) !== JSON.stringify(["-b:a", "96k"])) {
    throw new Error(`Unexpected ffmpeg args: ${JSON.stringify(start.config.audio.ffmpeg_args)}`);
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
