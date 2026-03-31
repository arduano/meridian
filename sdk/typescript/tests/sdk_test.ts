import { type MidiAnalysisData } from "../src/index.ts";
import { createDenoMeridianClient } from "../src/runtime/deno_client.ts";

function defaultExecutablePath(): string {
  const envPath = Deno.env.get("MERIDIAN_STDIO_BIN");
  if (envPath) {
    return envPath;
  }
  return new URL("../../../target/debug/meridian-stdio", import.meta.url)
    .pathname;
}

function defaultSoundfontPath(): string {
  return "/mnt/fat/Midis/Soundfonts/test.sfz";
}

async function ensureExecutable(path: string): Promise<void> {
  try {
    const stat = await Deno.stat(path);
    if (stat.isFile) {
      return;
    }
  } catch {
    // build below
  }

  const command = new Deno.Command("nix-shell", {
    cwd: new URL("../../..", import.meta.url).pathname,
    args: [
      "--run",
      "PATH=/run/current-system/sw/bin:$PATH cargo build -p meridian-stdio",
    ],
    stdout: "inherit",
    stderr: "inherit",
  });
  const status = await command.spawn().status;
  if (status.code !== 0) {
    throw new Error(`Failed to build meridian-stdio, exit code ${status.code}`);
  }
}

async function writeFixtureMidi(path: string): Promise<void> {
  const bytes = new Uint8Array([
    0x4d,
    0x54,
    0x68,
    0x64,
    0x00,
    0x00,
    0x00,
    0x06,
    0x00,
    0x00,
    0x00,
    0x01,
    0x00,
    0x60,
    0x4d,
    0x54,
    0x72,
    0x6b,
    0x00,
    0x00,
    0x00,
    0x1b,
    0x00,
    0xff,
    0x51,
    0x03,
    0x07,
    0xa1,
    0x20,
    0x00,
    0x90,
    0x3c,
    0x64,
    0x30,
    0x90,
    0x40,
    0x64,
    0x30,
    0x80,
    0x3c,
    0x40,
    0x30,
    0x80,
    0x40,
    0x40,
    0x00,
    0xff,
    0x2f,
    0x00,
  ]);
  await Deno.writeFile(path, bytes);
}

Deno.test("analysis job runs through the declarative SDK API", async () => {
  const executablePath = defaultExecutablePath();
  await ensureExecutable(executablePath);

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-analysis-" });
  const midiPath = `${tempDir}/fixture.mid`;
  await writeFixtureMidi(midiPath);

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

  const tempDir = await Deno.makeTempDir({ prefix: "meridian-sdk-plan-" });
  const midiPath = `${tempDir}/fixture.mid`;
  await writeFixtureMidi(midiPath);

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
  const midiA = `${tempDir}/a.mid`;
  const midiB = `${tempDir}/b.mid`;
  const output = `${tempDir}/out.mid`;
  await writeFixtureMidi(midiA);
  await writeFixtureMidi(midiB);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const events: string[] = [];
    const result = await client.modification.rangeSelect({
      inputs: [midiA, midiB],
      output,
      event_kinds: ["note"],
      config: {
        notes: {
          velocity_scale: 0.75,
        },
      },
      onEvent: (event) => events.push(event.type),
    });
    if (result.input_count !== 2) {
      throw new Error(`Expected 2 inputs, got ${result.input_count}`);
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
  const midiA = `${tempDir}/a.mid`;
  const midiB = `${tempDir}/b.mid`;
  const output = `${tempDir}/out.mid`;
  await writeFixtureMidi(midiA);
  await writeFixtureMidi(midiB);

  const client = await createDenoMeridianClient(executablePath);
  try {
    const task = client.modification.velocityMap.scale(0.5, {
      inputs: [midiA, midiB],
      output,
    });
    const handle = await task.start();
    const result = await handle.wait();

    if (result.input_count !== 2) {
      throw new Error(`Expected 2 inputs, got ${result.input_count}`);
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
  const midiPath = `${tempDir}/fixture.mid`;
  const output = `${tempDir}/out.wav`;
  await writeFixtureMidi(midiPath);
  await Deno.stat(defaultSoundfontPath());

  const client = await createDenoMeridianClient(executablePath);
  try {
    const events: string[] = [];
    const result = await client.audio.render({
      midiPath,
      output,
      sampleRate: 22050,
      channels: 2,
      soundfonts: [defaultSoundfontPath()],
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
