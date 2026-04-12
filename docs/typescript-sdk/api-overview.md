# API Overview

This page is intentionally high level. It maps the public SDK surface without
turning into per-function reference docs yet.

## Main Entry Points

### Package entrypoint

The package root is the primary SDK entrypoint:

- locally: [`sdk/typescript/src/index.ts`](../../sdk/typescript/src/index.ts)
- when published: `@meridian/cli-sdk`

It re-exports the stable client factories, helpers, protocol facade, and the
runtime-specific convenience entrypoints.

### Client creation

Core exports in the package entrypoint:

- `createMeridianClient(...)`
- `createProtocolClient(...)`
- `CreateClientOptions`

Runtime-specific helpers:

- Deno: [`runtime/deno_client.ts`](../../sdk/typescript/src/runtime/deno_client.ts)
- Node: [`runtime/node_client.ts`](../../sdk/typescript/src/runtime/node_client.ts)
- Bun: [`runtime/bun_client.ts`](../../sdk/typescript/src/runtime/bun_client.ts)

### High-level client

`MeridianClient` is the normal application entry point. Its surface groups into:

- `analysis(...)`
- `resources.*`
- `modification.*`
- `audio.render(...)`
- `video.render(...)`
- `close()`

Current `modification.*` helpers cover every modifier tool exposed by
`meridian-core`:

- `rangeSelect(...)`
- `tempoMap.flatten(...)`
- `tempoMap.scaleBpm(...)`
- `tempoMap.replace(...)`
- `timeWarp(...)`
- `channelRemap(...)`
- `trackRoute.collapseAll(...)`
- `trackRoute.splitByChannel(...)`
- `trackRoute.map(...)`
- `program(...)`
- `controlChange(...)`
- `pitchBend(...)`
- `velocityMap.scale(...)`
- `velocityMap.gamma(...)`
- `velocityMap.polyline(...)`
- `changePpq(...)`
- `extractTrack(...)`
- `noteLength(...)`
- `quantize(...)`
- `humanize(...)`
- `keyMap(...)`
- `metaText(...)`
- `sharedMetadataTrack(...)`
- `sysex(...)`

Detailed workflow docs:

- [MIDI Modification Tools](./midi-modification-tools.md)
- [Audio Rendering](./audio-rendering.md)
- [Video Rendering](./video-rendering.md)

### Lower-level protocol client

`MeridianProtocolClient` is the escape hatch for direct command/event work.

Primary responsibilities:

- spawn and manage the subprocess
- send typed protocol requests
- receive asynchronous events
- shut down cleanly

Detailed protocol workflow docs: TBD.

## Task And Handle Model

The high-level SDK does not expose everything as one-shot function calls.
Several workflows are modeled as tasks and job handles.

Current job/task concepts include:

- `MidiAnalysisTask` / `MidiAnalysisJobHandle`
- `MidiProcessTask` / `MidiProcessJobHandle`
- `AudioRenderTask` / `AudioRenderJobHandle`
- `VideoRenderTask` / `VideoRenderJobHandle`

Reference page for lifecycle semantics, progress events, and cancellation: TBD.

## Helpers And Builders

Helper exports live in [`sdk/typescript/src/helpers.ts`](../../sdk/typescript/src/helpers.ts).

Current helper areas:

- default processing config builders
- deep merge utilities
- `midiTools` convenience builders for modification workflows

`midiTools` now includes builders for every currently supported modifier tool,
including `sharedMetadataTrack(...)`.

Detailed helper docs: TBD.

## Protocol Types

Protocol types live in [`sdk/typescript/src/protocol.ts`](../../sdk/typescript/src/protocol.ts).
That file is the handwritten facade over `sdk/typescript/generated/*`.

This is the main place to look for:

- request/response shapes
- event unions
- modifier tool types
- analysis, audio, and video protocol types
- video container enums such as `VideoOutputContainer`
- scene config types for renderer-specific video customization

If you need the raw generated schema names, inspect
[`sdk/typescript/generated`](../../sdk/typescript/generated). If you want the
SDK-facing names callers should actually use, start in `protocol.ts`.

Schema/type reference docs: TBD.

## Suggested Reference Split

When this scaffold grows into full docs, the likely split is:

1. client creation and lifecycle
2. analysis workflows
3. MIDI modification workflows
4. audio rendering
5. video rendering
6. raw protocol usage
7. helper builders and config composition
8. protocol type reference
