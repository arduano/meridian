import { createDenoProtocolClient } from "../src/runtime/deno_client.ts";
import type { CoreEvent } from "../src/protocol.ts";
import {
  defaultExecutablePath,
  defaultSoundfontPath,
  ensureExecutable,
  hasCommand,
  resolveMidiFixture,
  TWO_NOTE_MIDI,
} from "./common.ts";

function waitForEvent<T extends CoreEvent["type"]>(
  client: Awaited<ReturnType<typeof createDenoProtocolClient>>,
  type: T,
  accept: (event: Extract<CoreEvent, { type: T }>) => boolean,
  timeoutMs = 20_000,
): Promise<Extract<CoreEvent, { type: T }>> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      unsubscribe();
      reject(new Error(`Timed out waiting for ${type}`));
    }, timeoutMs);
    let unsubscribe = () => {};
    unsubscribe = client.onEvent((event) => {
      if (event.type !== type) {
        return;
      }
      const typed = event as Extract<CoreEvent, { type: T }>;
      if (!accept(typed)) {
        return;
      }
      clearTimeout(timeout);
      unsubscribe();
      resolve(typed);
    });
  });
}

Deno.test("stdio protocol round-trips raw load and shutdown commands", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);
  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);

  const client = await createDenoProtocolClient(executablePath);
  try {
    const parsed = await client.request({
      type: "load_parsed_midi",
      path: midiPath,
    });
    if (parsed.length !== 1 || parsed[0]?.type !== "parsed_midi_loaded") {
      throw new Error(`Unexpected parsed midi response: ${JSON.stringify(parsed)}`);
    }

    const audio = await client.request({
      type: "load_audio_midi",
      path: midiPath,
    });
    if (audio.length !== 1 || audio[0]?.type !== "midi_loaded") {
      throw new Error(`Unexpected audio midi response: ${JSON.stringify(audio)}`);
    }

    const shutdown = await client.request({ type: "shutdown" });
    if (shutdown.length !== 1 || shutdown[0]?.type !== "shutdown_complete") {
      throw new Error(`Unexpected shutdown response: ${JSON.stringify(shutdown)}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("stdio protocol smoke tests video render", async () => {
  if (!(await hasCommand("ffmpeg"))) {
    console.warn("skipping video stdio smoke because ffmpeg is unavailable");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no4-the-little-party.mid",
    TWO_NOTE_MIDI,
  );
  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-video-" });
  const output = `${tempDir}/video.mp4`;

  const client = await createDenoProtocolClient(executablePath);
  try {
    const events = await client.request({
      type: "start_render_video",
      config: {
        midi_path: midiPath,
        output,
        fps: 4,
        width: 160,
        height: 90,
        renderer: "piano_trail_classic",
        scene: null,
        view_range: 2,
        time_space: null,
        first_key: null,
        last_key: null,
        ffmpeg_args: ["-y"],
      },
    });
    const status = events[0];
    if (
      events.length !== 1 ||
      !status ||
      status.type !== "video_render_status" ||
      status.status.state !== "running"
    ) {
      throw new Error(`Unexpected video start response: ${JSON.stringify(events)}`);
    }

    const finished = await waitForEvent(
      client,
      "video_render",
      (event) => event.event.type === "render_finished",
    );
    if (finished.event.type !== "render_finished") {
      throw new Error(`Unexpected video render event: ${JSON.stringify(finished)}`);
    }
    if (finished.event.output !== output) {
      throw new Error(
        `Expected video output ${output}, got ${finished.event.output}`,
      );
    }
    const stat = await Deno.stat(output);
    if (!stat.isFile || stat.size === 0) {
      throw new Error(`Expected non-empty video output at ${output}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("stdio protocol accepts explicit scene config for video render", async () => {
  if (!(await hasCommand("ffmpeg"))) {
    console.warn("skipping scene-config video stdio smoke because ffmpeg is unavailable");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no4-the-little-party.mid",
    TWO_NOTE_MIDI,
  );
  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-video-scene-" });
  const output = `${tempDir}/video-scene.mp4`;

  const client = await createDenoProtocolClient(executablePath);
  try {
    const events = await client.request({
      type: "start_render_video",
      config: {
        midi_path: midiPath,
        output,
        fps: 4,
        width: 160,
        height: 90,
        renderer: "piano_trail_classic",
        scene: {
          scene_type: "three_d",
          projector: "piano_trail_classic",
          same_width_notes: true,
          fov: Math.PI / 3,
          view_height: 0.55,
          view_offset: 0.45,
          view_pan: 0.12,
          cam_ang: 0.68,
          cam_rot: 0.04,
          cam_spin: 0,
          viewdist: 14,
          viewback: 0.2,
          vertical_notes: false,
          note_down_speed: 0.6,
          note_up_speed: 0.2,
          box_notes: true,
          light_shade: false,
          show_keyboard: true,
          tilt_keys: true,
          eat_notes: false,
          aura_strength: 0.25,
          aura_enabled: true,
          notes_change_size: false,
          notes_change_tint: true,
          use_vel: false,
          palette: {
            source: "default_track_colors",
          },
          aura_image: {
            source: "builtin",
            name: "ring",
          },
        },
        view_range: 2,
        time_space: null,
        first_key: null,
        last_key: null,
        ffmpeg_args: ["-y"],
      },
    });
    const status = events[0];
    if (
      events.length !== 1 ||
      !status ||
      status.type !== "video_render_status" ||
      status.status.state !== "running"
    ) {
      throw new Error(`Unexpected video start response: ${JSON.stringify(events)}`);
    }

    const finished = await waitForEvent(
      client,
      "video_render",
      (event) => event.event.type === "render_finished",
    );
    if (finished.event.type !== "render_finished") {
      throw new Error(`Unexpected video render event: ${JSON.stringify(finished)}`);
    }
    if (finished.event.output !== output) {
      throw new Error(
        `Expected video output ${output}, got ${finished.event.output}`,
      );
    }
    const stat = await Deno.stat(output);
    if (!stat.isFile || stat.size === 0) {
      throw new Error(`Expected non-empty video output at ${output}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("stdio protocol smoke tests audio render when a soundfont is available", async () => {
  const soundfont = await defaultSoundfontPath();
  if (!soundfont) {
    console.warn("skipping audio stdio smoke because no bundled or override soundfont is available");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no13-consolation.mid",
    TWO_NOTE_MIDI,
  );
  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-audio-" });
  const output = `${tempDir}/audio.wav`;

  const client = await createDenoProtocolClient(executablePath);
  try {
    const load = await client.request({
      type: "load_audio_midi",
      path: midiPath,
    });
    if (load.length !== 1 || load[0]?.type !== "midi_loaded") {
      throw new Error(`Unexpected audio load response: ${JSON.stringify(load)}`);
    }

    const start = await client.request({
      type: "start_render_audio",
      config: {
        midi_path: midiPath,
        output,
        sample_rate: 22_050,
        channels: 2,
        use_limiter: false,
        soundfonts: [soundfont],
      },
    });
    const status = start[0];
    if (
      start.length !== 1 ||
      !status ||
      status.type !== "audio_render_status" ||
      status.status.state !== "running"
    ) {
      throw new Error(`Unexpected audio start response: ${JSON.stringify(start)}`);
    }

    const finished = await waitForEvent(
      client,
      "audio_render",
      (event) => event.event.type === "render_finished",
    );
    if (finished.event.type !== "render_finished") {
      throw new Error(`Unexpected audio render event: ${JSON.stringify(finished)}`);
    }
    const bytes = await Deno.readFile(output);
    if (
      bytes.length < 12 ||
      String.fromCharCode(...bytes.slice(0, 4)) !== "RIFF"
    ) {
      throw new Error(`Expected RIFF audio output at ${output}`);
    }
  } finally {
    await client.close();
  }
});
