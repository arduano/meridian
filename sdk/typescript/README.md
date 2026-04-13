# TypeScript SDK

This package is the beginner-facing TypeScript entrypoint for Meridian.

Start with:

- [`mod.ts`](./mod.ts): the stable public entrypoint for repo-local use and tagged GitHub imports
- [`src/index.ts`](./src/index.ts): the implementation entrypoint behind `mod.ts`
- [`examples`](./examples): runnable end-to-end examples
- [`../../docs/typescript-sdk`](../../docs/typescript-sdk): deeper workflow docs

## Quick Start

Repo-local Deno usage:

```ts
import { createDenoMeridianClient } from "./mod.ts";

const client = await createDenoMeridianClient("./target/debug/meridian-cli");

try {
  const analysis = await client.analysis("./song.mid", {
    file: true,
    summary: true,
    notes: true,
  });

  console.log(analysis.total_notes);
} finally {
  await client.close();
}
```

Run it with:

```bash
deno run --allow-env --allow-read --allow-write --allow-run your_script.ts
```

## Start Here

- [`examples/analysis.ts`](./examples/analysis.ts): load and analyze a MIDI file
- [`examples/process.ts`](./examples/process.ts): apply one MIDI modifier tool
- [`examples/audio_render.ts`](./examples/audio_render.ts): render audio
- [`examples/video_render.ts`](./examples/video_render.ts): render video
- [`examples/raw_protocol.ts`](./examples/raw_protocol.ts): use the lower-level
  protocol client directly

## Public Surface

- `createDenoMeridianClient`, `createNodeMeridianClient`,
  `createBunMeridianClient` Normal high-level clients.
- `createDenoProtocolClient`, `createNodeProtocolClient`,
  `createBunProtocolClient` Lower-level raw stdio protocol clients.
- `MeridianClient` Workflow-oriented SDK surface for resources, display,
  modification, audio, and video.
- `midiTools` Typed builders for every MIDI modifier tool.
- `protocol.ts` exports The public schema and protocol type facade.

## GitHub Release Versioning

The SDK is versioned by Git tag, not by a registry publish. The stable remote
import shape is:

```ts
import { createDenoMeridianClient } from "https://raw.githubusercontent.com/<owner>/meridian/v0.1.0/sdk/typescript/mod.ts";
```

Pin tags or commits. Do not import from `main`.

## Validation

```bash
deno check mod.ts src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts tests/client_internal_test.ts
deno test --allow-env --allow-read --allow-write --allow-run tests
```

## More Docs

- [`../../docs/typescript-sdk/getting-started.md`](../../docs/typescript-sdk/getting-started.md)
- [`../../docs/typescript-sdk/api-overview.md`](../../docs/typescript-sdk/api-overview.md)
- [`../../docs/typescript-sdk/audio-rendering.md`](../../docs/typescript-sdk/audio-rendering.md)
- [`../../docs/typescript-sdk/video-rendering.md`](../../docs/typescript-sdk/video-rendering.md)
- [`../../docs/typescript-sdk/midi-modification-tools.md`](../../docs/typescript-sdk/midi-modification-tools.md)
