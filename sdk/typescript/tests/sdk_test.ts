import { type MidiAnalysisData, type SceneConfig } from "../src/index.ts";
import { createDenoMeridianClient } from "../src/runtime/deno_client.ts";
import {
  defaultExecutablePath,
  defaultSoundfontPath,
  ensureExecutable,
  hasCommand,
  resolveMidiFixture,
  TWO_NOTE_MIDI,
} from "./common.ts";

Deno.test("analysis job runs through the declarative SDK API", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const progress: number[] = [];
    const analysis: MidiAnalysisData = await client.analysis(midiPath, {
      file: true,
      summary: true,
      events: true,
      notes: true,
      tempo: true,
      buckets: 4,
      onProgress: (update) => progress.push(update.progress),
    });
    if (analysis.total_notes !== 2) {
      throw new Error(`Expected 2 notes, got ${analysis.total_notes}`);
    }
    if (analysis.events.note_on_events !== 2) {
      throw new Error(
        `Expected 2 note-on events, got ${analysis.events.note_on_events}`,
      );
    }
    if (analysis.buckets.length !== 4) {
      throw new Error(`Expected 4 buckets, got ${analysis.buckets.length}`);
    }
    if (progress.length === 0) {
      throw new Error("Expected at least one analysis progress update");
    }
  } finally {
    await client.close();
  }
});

Deno.test("analysis start returns a live job handle", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const task = client.analysis(midiPath, {
      file: true,
      summary: true,
      notes: true,
      buckets: 3,
    });
    const handle = await task.start();
    const analysis = await handle.wait();

    if (analysis.total_notes !== 2) {
      throw new Error(`Expected 2 notes, got ${analysis.total_notes}`);
    }
    if (analysis.buckets.length !== 3) {
      throw new Error(`Expected 3 buckets, got ${analysis.buckets.length}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("midi processing job runs through the SDK", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-process-" });
  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);
  const output = `${tempDir}/out.mid`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const events: string[] = [];
    const result = await client.modification.keyMap({
      input: midiPath,
      output,
      mappings: [{ from: 60, to: 64 }],
      onEvent: (event) => events.push(event.type),
    });
    if (result.input !== midiPath) {
      throw new Error(`Expected input ${midiPath}, got ${result.input}`);
    }
    if (result.output_track_count < 1) {
      throw new Error(
        `Expected at least 1 output track, got ${result.output_track_count}`,
      );
    }
    await Deno.stat(output);
    if (!events.includes("process_finished")) {
      throw new Error(
        `Expected process_finished in event stream, got ${events.join(", ")}`,
      );
    }
  } finally {
    await client.close();
  }
});

Deno.test("midi processing start returns a live job handle", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-process-handle-" });
  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);
  const output = `${tempDir}/out.mid`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const task = client.modification.keyMap({
      input: midiPath,
      output,
      mappings: [{ from: 60, to: 67 }],
    });
    const handle = await task.start();
    const result = await handle.wait();

    if (result.input !== midiPath) {
      throw new Error(`Expected input ${midiPath}, got ${result.input}`);
    }
    await Deno.stat(output);
  } finally {
    await client.close();
  }
});

Deno.test("shared metadata track helper is exposed through the SDK", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({
    prefix: "meridian-sdk-shared-metadata-",
  });
  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);
  const output = `${tempDir}/out.mid`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const result = await client.modification.sharedMetadataTrack({
      input: midiPath,
      output,
      destination: { mode: "create_new" },
      move_tempo_events: true,
    });

    if (result.output_track_count < 2) {
      throw new Error(
        `Expected shared metadata track to create at least 2 tracks, got ${result.output_track_count}`,
      );
    }
    await Deno.stat(output);
  } finally {
    await client.close();
  }
});

Deno.test("midi merge runs through the SDK", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-merge-" });
  const midiA = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);
  const midiB = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);
  const output = `${tempDir}/merged.mid`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const result = await client.merge.midiFiles({
      inputs: [midiA, midiB],
      output,
      config: {
        mode: "append_tracks",
      },
    });

    if (result.input_count !== 2) {
      throw new Error(`Expected 2 inputs, got ${result.input_count}`);
    }
    if (result.output_track_count < 2) {
      throw new Error(
        `Expected at least 2 output tracks, got ${result.output_track_count}`,
      );
    }
    await Deno.stat(output);
  } finally {
    await client.close();
  }
});

Deno.test("audio render runs through the declarative SDK API", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-audio-" });
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no13-consolation.mid",
    TWO_NOTE_MIDI,
  );
  const output = `${tempDir}/out.wav`;
  const soundfontPath = await defaultSoundfontPath();
  if (!soundfontPath) {
    throw new Error("No bundled or override soundfont is available for audio render smoke");
  }
  await Deno.stat(soundfontPath);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const events: string[] = [];
    const result = await client.audio.render({
      midiPath,
      output,
      sampleRate: 22050,
      channels: 2,
      soundfonts: [soundfontPath],
      onEvent: (event) => events.push(event.type),
    });

    if (result.frames_written <= 0) {
      throw new Error(`Expected frames_written > 0, got ${result.frames_written}`);
    }
    await Deno.stat(output);
    if (!events.includes("render_finished")) {
      throw new Error(
        `Expected render_finished in event stream, got ${events.join(", ")}`,
      );
    }
  } finally {
    await client.close();
  }
});

Deno.test("video render runs through the declarative SDK API", async () => {
  if (!(await hasCommand("ffmpeg"))) {
    console.warn("skipping video render smoke because ffmpeg is unavailable");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-video-" });
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no4-the-little-party.mid",
    TWO_NOTE_MIDI,
  );
  const output = `${tempDir}/out.mkv`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const events: string[] = [];
    const result = await client.video.render({
      midiPath,
      output,
      container: "mkv",
      fps: 4,
      width: 160,
      height: 90,
      renderer: "piano_trail_classic",
      viewRange: 2,
      ffmpegArgs: ["-y"],
      onEvent: (event) => events.push(event.type),
    });

    if (result.output !== output) {
      throw new Error(`Expected output ${output}, got ${result.output}`);
    }
    const stat = await Deno.stat(output);
    if (!stat.isFile || stat.size === 0) {
      throw new Error(`Expected non-empty video output at ${output}`);
    }
    if (!events.includes("render_finished")) {
      throw new Error(
        `Expected render_finished in event stream, got ${events.join(", ")}`,
      );
    }
  } finally {
    await client.close();
  }
});

Deno.test("video render runs through the declarative SDK API with muxed audio", async () => {
  if (!(await hasCommand("ffmpeg"))) {
    console.warn("skipping muxed video render smoke because ffmpeg is unavailable");
    return;
  }

  const soundfont = await defaultSoundfontPath();
  if (!soundfont) {
    console.warn("skipping muxed video render smoke because no soundfont is available");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-video-muxed-" });
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no4-the-little-party.mid",
    TWO_NOTE_MIDI,
  );
  const output = `${tempDir}/out-muxed.mkv`;

  const client = await createDenoMeridianClient(executablePath);
  try {
    const result = await client.video.render({
      midiPath,
      output,
      container: "mkv",
      fps: 4,
      width: 160,
      height: 90,
      renderer: "piano_trail_classic",
      viewRange: 2,
      ffmpegArgs: ["-y"],
      audio: {
        sampleRate: 22_050,
        channels: 2,
        useLimiter: true,
        soundfonts: [soundfont],
        ffmpegArgs: ["-b:a", "96k"],
      },
    });

    if (result.output !== output) {
      throw new Error(`Expected muxed output ${output}, got ${result.output}`);
    }
    const stat = await Deno.stat(output);
    if (!stat.isFile || stat.size === 0) {
      throw new Error(`Expected non-empty muxed video output at ${output}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("video render accepts full scene config through the SDK API", async () => {
  if (!(await hasCommand("ffmpeg"))) {
    console.warn("skipping scene-config video render smoke because ffmpeg is unavailable");
    return;
  }

  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-video-scene-" });
  const midiPath = await resolveMidiFixture(
    "piano/burgmuller-op100-no4-the-little-party.mid",
    TWO_NOTE_MIDI,
  );
  const output = `${tempDir}/out-scene.mp4`;

  const scene: SceneConfig = {
    scene_type: "three_d",
    projector: "piano_trail_classic",
    background: {
      source: "none",
    },
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
  };

  const client = await createDenoMeridianClient(executablePath);
  try {
    const result = await client.video.render({
      midiPath,
      output,
      fps: 4,
      width: 160,
      height: 90,
      scene,
      viewRange: 2,
      ffmpegArgs: ["-y"],
    });

    if (result.output !== output) {
      throw new Error(`Expected output ${output}, got ${result.output}`);
    }
    const stat = await Deno.stat(output);
    if (!stat.isFile || stat.size === 0) {
      throw new Error(`Expected non-empty video output at ${output}`);
    }
  } finally {
    await client.close();
  }
});

Deno.test("resource helpers load parsed and audio midi through the SDK", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const midiPath = await resolveMidiFixture("smoke-two-notes.mid", TWO_NOTE_MIDI);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const parsedMidiId = await client.resources.loadParsedMidi(midiPath);
    const loadedMidi = await client.resources.loadAudioMidi(midiPath);

    if (typeof parsedMidiId !== "number") {
      throw new Error(`Expected numeric parsed midi id, got ${parsedMidiId}`);
    }
    if (loadedMidi.path !== midiPath) {
      throw new Error(`Expected loaded path ${midiPath}, got ${loadedMidi.path}`);
    }
  } finally {
    await client.close();
  }
});
