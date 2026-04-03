# Examples

The current examples live in [`sdk/typescript/examples`](../../sdk/typescript/examples).
They are small and practical, which makes them the best current source of usage
patterns until the rest of the docs are filled in.

## Current Example Files

- [`analysis.ts`](../../sdk/typescript/examples/analysis.ts): basic MIDI
  analysis flow
- [`resources.ts`](../../sdk/typescript/examples/resources.ts): explicit
  resource loading
- [`process.ts`](../../sdk/typescript/examples/process.ts): MIDI modification
  flow
- [`audio_render.ts`](../../sdk/typescript/examples/audio_render.ts): audio
  rendering flow
- [`video_render.ts`](../../sdk/typescript/examples/video_render.ts): video
  rendering flow
- [`video_render_scene.ts`](../../sdk/typescript/examples/video_render_scene.ts):
  video rendering with full scene-config passthrough
- [`raw_protocol.ts`](../../sdk/typescript/examples/raw_protocol.ts): direct
  protocol usage
- [`_shared.ts`](../../sdk/typescript/examples/_shared.ts): shared helper setup
  for local examples

## Documentation Plan

Each example should eventually gain:

- a short explanation of what it demonstrates
- the required environment and inputs
- expected output artifacts
- the related API surface

For now, this page is mainly a navigation hub.

Related deep-dive pages:

- [Audio Rendering](./audio-rendering.md)
- [Video Rendering](./video-rendering.md)
