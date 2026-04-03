# TypeScript SDK

This section is the initial docs scaffold for the Meridian TypeScript SDK in
[`sdk/typescript`](../../sdk/typescript).

The SDK is currently documented as **Deno-first** because the existing examples,
checks, and tests are centered on Deno. The package surface is intentionally
runtime-aware, though, and the repository already includes runtime adapters for
Node and Bun. In practice, that means the SDK should be portable anywhere a
runtime can:

- spawn `meridian-cli`
- exchange line-delimited JSON over stdio
- read and write normal filesystem paths

## Start Here

- [Getting Started](./getting-started.md): requirements, permissions,
  installation shape, initialization, and first client setup
- [Audio Rendering](./audio-rendering.md): soundfont requirements, WAV render
  flow, progress events, and task lifecycle
- [Video Rendering](./video-rendering.md): renderer selection, `ffmpeg`
  requirements, progress events, and output workflow
- [Runtime Support](./runtime-support.md): what is Deno-centric today and what
  is expected to work in other runtimes
- [API Overview](./api-overview.md): high-level SDK surface map without
  per-function deep dives
- [Examples](./examples.md): current example entry points in the repository

## Current Status

- Package name: `@meridian/cli-sdk`
- Source root: [`sdk/typescript/src`](../../sdk/typescript/src)
- Examples: [`sdk/typescript/examples`](../../sdk/typescript/examples)
- Today’s docs assume repo-local development first, with package-manager
  distribution documentation to be filled in later
