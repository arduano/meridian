# Audio Rendering

Audio rendering is the SDK path for turning a MIDI file into an offline audio
file. In the current implementation, this path is oriented around writing a WAV
output file.

This page goes deeper than the general SDK overview because audio rendering has
real environment requirements and a slightly richer lifecycle than a simple
one-shot helper.

## What `client.audio.render(...)` Does

At the high level:

1. the SDK ensures the MIDI is loaded for audio use
2. it sends a `start_render_audio` protocol request
3. Meridian starts an offline render job
4. progress events stream back over stdio
5. the task resolves when a `render_finished` event arrives

The public entry point is:

```ts
const task = client.audio.render({
  midiPath,
  output,
  soundfonts: [soundfontPath],
});
```

That returns an `AudioRenderTask`, which is promise-like. You can either:

- `await` it directly for the finished event
- call `.start()` to get an `AudioRenderJobHandle`

## Requirements

### MIDI input

You need a readable MIDI file path. The SDK passes the path to Meridian and
loads it into the audio side before starting the render.

### Output path

You need a writable output path. Use a `.wav` path unless you have a specific
reason to experiment otherwise.

### Soundfonts

Audio rendering needs usable soundfont assets. In this repo, examples resolve a
bundled SFZ soundfont under `assets/soundfonts` and also allow override through
environment variables such as `MERIDIAN_EXAMPLE_SOUNDFONT` or
`MERIDIAN_SOUNDFONT`.

Practically:

- if you pass `soundfonts`, those paths are used for the render job
- if you do not pass `soundfonts`, Meridian falls back to its current audio
  configuration
- if the effective soundfont setup is invalid, the render will fail

For portable application code, treat soundfont selection as an explicit app
responsibility instead of assuming a bundled default exists.

### Runtime permissions

For Deno, expect the render path to need:

- `--allow-run`
- `--allow-read`
- `--allow-write`
- `--allow-env` if you read soundfont or binary paths from env vars

## Minimal Render

```ts
import { createDenoMeridianClient } from "../sdk/typescript/src/runtime/deno_client.ts";

const client = await createDenoMeridianClient("./target/debug/meridian-cli");

try {
  const result = await client.audio.render({
    midiPath: "./song.mid",
    output: "./song.wav",
    soundfonts: ["./assets/piano.sfz"],
  });

  console.log(result.frames_written);
} finally {
  await client.close();
}
```

The resolved value is the final `render_finished` event, which includes:

- `output`
- `frames_written`
- `rendered_seconds`

## Common Options

`AudioRenderOptions` currently supports:

| Field | Required | Purpose |
| --- | --- | --- |
| `midiPath` | Yes | Source MIDI file |
| `output` | Yes | Output audio file path |
| `sampleRate` | No | Override sample rate |
| `channels` | No | Override channel count |
| `useLimiter` | No | Override limiter behavior |
| `soundfonts` | No | Explicit soundfont file list |
| `onEvent` | No | Subscribe to render events during the job |

A slightly more explicit example:

```ts
const result = await client.audio.render({
  midiPath: "./song.mid",
  output: "./song.wav",
  sampleRate: 44100,
  channels: 2,
  useLimiter: true,
  soundfonts: ["./assets/piano.sfz"],
});
```

## Progress Events

Audio render events are event-oriented rather than frame-oriented. The main
progress callback is `render_progress`.

Useful fields include:

- `event_index`
- `total_events`
- `time_seconds`
- `rendered_seconds`
- `frames_written`
- `voice_count`

Example:

```ts
const result = await client.audio.render({
  midiPath: "./song.mid",
  output: "./song.wav",
  soundfonts: ["./assets/piano.sfz"],
  onEvent(event) {
    if (event.type === "render_progress") {
      console.log({
        eventIndex: event.event_index,
        totalEvents: event.total_events,
        renderedSeconds: event.rendered_seconds,
        framesWritten: event.frames_written,
        voiceCount: event.voice_count,
      });
    }
  },
});
```

The full event lifecycle currently includes:

- `render_started`
- `render_progress`
- `render_finished`
- `render_cancelled`
- `render_failed`

## Task vs Handle

If all you need is completion, `await client.audio.render(...)` is enough.

If you need cancellation or status refresh, use the job handle:

```ts
const task = client.audio.render({
  midiPath: "./song.mid",
  output: "./song.wav",
  soundfonts: ["./assets/piano.sfz"],
});

const handle = await task.start();

handle.onEvent((event) => {
  if (event.type === "render_progress") {
    console.log(event.frames_written);
  }
});

const status = await handle.refreshStatus();
console.log(status);

const finished = await handle.wait();
console.log(finished.output);
```

Use the handle when you need:

- event subscription after task construction
- polling with `refreshStatus()`
- explicit cancellation with `cancel()`

## Cancellation

Audio render handles support cancellation:

```ts
const task = client.audio.render({
  midiPath: "./song.mid",
  output: "./song.wav",
  soundfonts: ["./assets/piano.sfz"],
});

const handle = await task.start();

setTimeout(() => {
  void handle.cancel();
}, 1000);

try {
  await handle.wait();
} catch (error) {
  console.log("render cancelled", error);
}
```

In practice, a cancelled job rejects with a subprocess-level error rather than
resolving as a successful finished render, so wrap cancellation flows in
`try`/`catch`.

## Status Model

`refreshStatus()` returns one of these states:

- `idle`
- `running`
- `cancelling`

When running or cancelling, status includes:

- `output`
- `total_events`
- `event_index`
- `time_seconds`
- `rendered_seconds`
- `frames_written`

This is useful for coarse polling, but the event stream is usually the better
source of detailed progress.

## Concurrency Notes

Within one Meridian CLI process:

- only one audio render job can be active at a time
- trying to start another audio render while one is active returns an error

If you need parallel audio renders, plan around multiple client/subprocess
instances instead of one shared client.

## Failure Modes To Expect

Common failure classes:

- invalid `meridian-cli` path
- unreadable MIDI input
- unwritable output path
- missing or broken soundfont path
- host audio backend/config issues surfaced by the CLI

The final failure arrives as a `render_failed` event and is surfaced through the
task/handle as an error.

## Recommended Pattern

For most applications:

1. resolve the CLI path once
2. resolve the soundfont path once
3. pass an explicit `.wav` output path
4. use `onEvent` for logging or UI progress
5. always close the client

## Related Files

- [`sdk/typescript/examples/audio_render.ts`](../../sdk/typescript/examples/audio_render.ts)
- [`sdk/typescript/src/internal/client.ts`](../../sdk/typescript/src/internal/client.ts)
- [`sdk/typescript/generated/AudioRenderEvent.ts`](../../sdk/typescript/generated/AudioRenderEvent.ts)
- [`sdk/typescript/generated/AudioRenderStatus.ts`](../../sdk/typescript/generated/AudioRenderStatus.ts)
