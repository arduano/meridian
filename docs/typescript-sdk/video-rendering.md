# Video Rendering

Video rendering is the SDK path for turning a MIDI file into a rendered video
artifact. Compared with audio rendering, this path has more host-specific
requirements because it depends on both Meridian’s renderer stack and external
video encoding.

## What `client.video.render(...)` Does

At the high level:

1. the SDK sends a `start_render_video` protocol request
2. Meridian renders frames with the selected renderer
3. raw frames are piped into `ffmpeg`
4. progress events stream back over stdio
5. the task resolves when a `render_finished` event arrives

The public entry point is:

```ts
const task = client.video.render({
  midiPath,
  output,
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "pfa",
});
```

That returns a `VideoRenderTask`, which is also promise-like.

## Requirements

### MIDI input

You need a readable MIDI file path.

### Output path

You need a writable video output path. In practice, choose an extension that
matches what you expect `ffmpeg` to write, such as `.mp4`.

### `ffmpeg`

Video rendering depends on `ffmpeg` being available on the host. The repository
tests explicitly skip video render smoke tests when `ffmpeg` is unavailable.

That means video rendering should be treated as unavailable unless the runtime
environment can successfully spawn `ffmpeg`.

### Renderer support

The SDK currently exposes these renderer identifiers:

- `"flat"`
- `"pfa"`
- `"piano_trail_classic"`

Those map to Meridian renderer implementations on the CLI side. Choose the
renderer explicitly in application code instead of relying on defaults.

### Runtime permissions

For Deno, expect the render path to need:

- `--allow-run`
- `--allow-read`
- `--allow-write`
- `--allow-env` if your application resolves paths from env vars

## Minimal Render

```ts
import { createDenoMeridianClient } from "../sdk/typescript/src/runtime/deno_client.ts";

const client = await createDenoMeridianClient("./target/debug/meridian-cli");

try {
  const result = await client.video.render({
    midiPath: "./song.mid",
    output: "./song.mp4",
    fps: 24,
    width: 1280,
    height: 720,
    renderer: "pfa",
  });

  console.log(result.total_frames);
} finally {
  await client.close();
}
```

The resolved value is the final `render_finished` event, which includes:

- `total_frames`
- `elapsed_seconds`
- `average_fps`
- `output`

## Common Options

`VideoRenderOptions` currently supports:

| Field | Required | Purpose |
| --- | --- | --- |
| `midiPath` | Yes | Source MIDI file |
| `output` | Yes | Output video file path |
| `fps` | Yes | Output frame rate |
| `width` | Yes | Output width in pixels |
| `height` | Yes | Output height in pixels |
| `renderer` | No | Renderer kind when you are not passing a full scene |
| `scene` | No | Full renderer-specific scene config passthrough |
| `viewRange` | No | Scene view range |
| `timeSpace` | No | `"time"` or `"tick"` |
| `firstKey` | No | Lower piano key bound |
| `lastKey` | No | Upper piano key bound |
| `ffmpegArgs` | No | Extra `ffmpeg` arguments |
| `onEvent` | No | Subscribe to render events during the job |

More explicit example:

```ts
const result = await client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 30,
  width: 1920,
  height: 1080,
  renderer: "piano_trail_classic",
  viewRange: 4,
  timeSpace: "time",
  firstKey: 21,
  lastKey: 108,
  ffmpegArgs: ["-pix_fmt", "yuv420p"],
});
```

## Renderer Choice

Renderer selection is not cosmetic. It affects how the CLI builds the visual
scene for each frame.

Current values:

- `flat`
- `pfa`
- `piano_trail_classic`

The repository examples use `flat` for a lightweight path, while the SDK tests
exercise `piano_trail_classic` as well.

If you are documenting presets later, this page is the right place for that
matrix.

## Full Scene Config Passthrough

If you need renderer-specific controls such as PFA top-bar color or
Piano Trail Classic camera and aura settings, pass a full `scene` object.

That is the escape hatch for per-renderer configuration. It carries the same
typed scene structure the core renderer uses internally.

Example:

```ts
import type { SceneConfig } from "@meridian/cli-sdk";

const scene: SceneConfig = {
  scene_type: "three_d",
  projector: "piano_trail_classic",
  same_width_notes: true,
  fov: Math.PI / 3,
  view_height: 0.58,
  view_offset: 0.52,
  view_pan: 0.18,
  cam_ang: 0.72,
  cam_rot: 0.06,
  cam_spin: 0,
  viewdist: 14,
  viewback: 0.25,
  vertical_notes: false,
  note_down_speed: 0.7,
  note_up_speed: 0.22,
  box_notes: true,
  light_shade: false,
  show_keyboard: true,
  tilt_keys: true,
  eat_notes: false,
  aura_strength: 0.35,
  aura_enabled: true,
  notes_change_size: false,
  notes_change_tint: true,
  use_vel: false,
  palette: {
    source: "zenith_palette",
    palette: { kind: "random_gradients" },
    randomize: true,
  },
  aura_image: {
    source: "builtin",
    name: "ring",
  },
};

const result = await client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  scene,
  viewRange: 4,
  timeSpace: "time",
});
```

The important rule is:

- if `scene` is provided, it defines the renderer-specific scene setup
- if `scene` is omitted, `renderer` builds a default scene for that renderer
- in the high-level TypeScript SDK, `renderer` may be omitted when `scene` is
  present; the client derives a compatible renderer tag internally for the
  protocol request

That means `scene` is the path to use for things like:

- PFA `top_bar_color`
- PFA note border width and keyboard layout toggles
- Piano Trail Classic camera position and angles
- Piano Trail Classic aura image and aura strength
- palette source and palette-specific configuration

## Time And Keyboard Windowing

Video rendering has a few scene-shaping controls that are worth documenting up
front:

- `viewRange`: controls how much of the scene is visible
- `timeSpace`: chooses whether the scene advances in `"time"` or `"tick"`
- `firstKey` / `lastKey`: constrain the visible keyboard range

These are useful when you want a tighter crop or a more piano-roll-like output
without changing the source MIDI.

## `ffmpegArgs`

`ffmpegArgs` lets the caller pass additional `ffmpeg` arguments through the
render request.

Example:

```ts
const result = await client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "flat",
  ffmpegArgs: ["-pix_fmt", "yuv420p", "-y"],
});
```

This is useful for:

- overwrite behavior during development
- pixel format compatibility
- encoder tuning

The `render_started` event includes `ffmpeg_command`, which makes it possible to
log the exact command Meridian launched.

## Progress Events

Video render progress is frame-oriented. The main progress callback is
`render_progress`.

Useful fields include:

- `frame_index`
- `total_frames`
- `current_time`
- `elapsed_seconds`
- `average_fps`

Example:

```ts
const result = await client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "flat",
  onEvent(event) {
    switch (event.type) {
      case "render_started":
        console.log(event.ffmpeg_command);
        break;
      case "render_progress":
        console.log({
          frame: event.frame_index,
          totalFrames: event.total_frames,
          averageFps: event.average_fps,
        });
        break;
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

Use the task directly when you only care about completion:

```ts
const finished = await client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "flat",
});
```

Use the handle when you need status polling or cancellation:

```ts
const task = client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "flat",
});

const handle = await task.start();

handle.onEvent((event) => {
  if (event.type === "render_progress") {
    console.log(event.frame_index);
  }
});

const status = await handle.refreshStatus();
console.log(status);

const finished = await handle.wait();
console.log(finished.output);
```

## Cancellation

Video render handles support cancellation:

```ts
const task = client.video.render({
  midiPath: "./song.mid",
  output: "./song.mp4",
  fps: 24,
  width: 1280,
  height: 720,
  renderer: "flat",
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

As with audio rendering, cancellation is not a successful completion path. Wrap
this in `try`/`catch` if cancellation is expected.

## Status Model

`refreshStatus()` returns one of:

- `idle`
- `running`
- `cancelling`

Running and cancelling states include:

- `output`
- `fps`
- `width`
- `height`
- `total_frames`
- `frame_index`
- `current_time`
- `elapsed_seconds`

That is useful for polling, but the event stream is still the better source of
user-facing progress updates.

## Concurrency Notes

Within one Meridian CLI process:

- only one video render job can be active at a time
- trying to start another video render while one is active returns an error

If you need multiple concurrent video renders, use multiple client/subprocess
instances.

## Failure Modes To Expect

Common failure classes:

- invalid `meridian-cli` path
- unreadable MIDI input
- unwritable output path
- `ffmpeg` missing from the host
- invalid extra `ffmpeg` arguments
- renderer or GPU/platform constraints surfaced by the CLI

The final failure arrives as a `render_failed` event and is surfaced through the
task/handle as an error.

## Recommended Pattern

For most applications:

1. detect `ffmpeg` availability before exposing video export
2. choose a renderer explicitly, or pass a full `scene` when you need
   renderer-specific styling
3. choose a conservative output format such as `.mp4`
4. log `render_started.ffmpeg_command` for debugging
5. use `onEvent` for progress reporting
6. always close the client

## Related Files

- [`sdk/typescript/examples/video_render.ts`](../../sdk/typescript/examples/video_render.ts)
- [`sdk/typescript/examples/video_render_scene.ts`](../../sdk/typescript/examples/video_render_scene.ts)
- [`sdk/typescript/src/internal/client.ts`](../../sdk/typescript/src/internal/client.ts)
- [`sdk/typescript/generated/VideoRenderEvent.ts`](../../sdk/typescript/generated/VideoRenderEvent.ts)
- [`sdk/typescript/generated/VideoRenderStatus.ts`](../../sdk/typescript/generated/VideoRenderStatus.ts)
