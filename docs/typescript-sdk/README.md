# TypeScript SDK

This section documents the Meridian TypeScript SDK in
[`sdk/typescript`](../../sdk/typescript).

The canonical local entrypoint is
[`sdk/typescript/src/index.ts`](../../sdk/typescript/src/index.ts). When
consumed as `@meridian/cli-sdk`, the package root exposes the same public
surface.

The SDK docs use Deno for the runnable examples because that is the easiest
repo-local path to validate. The package surface itself is runtime-aware, and
the repository already includes runtime adapters for Node and Bun. In practice,
that means the SDK should work anywhere a runtime can:

- spawn `meridian-cli`
- exchange line-delimited JSON over stdio
- read and write normal filesystem paths

## Start Here

- [Getting Started](./getting-started.md): requirements, permissions,
  installation shape, initialization, and first client setup
- [`sdk/typescript/README.md`](../../sdk/typescript/README.md): package-root
  quick start with the canonical beginner examples
- [MIDI Modification Tools](./midi-modification-tools.md): every current
  modifier tool, with TypeScript snippets, runnable examples, and implementation
  limits
- [Audio Rendering](./audio-rendering.md): soundfont requirements, WAV/FLAC/MP3
  render flow, progress events, and task lifecycle
- [Video Rendering](./video-rendering.md): renderer selection, `ffmpeg`
  requirements, progress events, and output workflow
- [Runtime Support](./runtime-support.md): runtime adapters, portability
  boundary, and repo-local guidance for Deno, Node, and Bun
- [API Overview](./api-overview.md): high-level SDK surface map without
  per-function deep dives
- [Examples](./examples.md): runnable TypeScript example entry points in the
  repository

## Where Things Live

- [`sdk/typescript/src`](../../sdk/typescript/src): the public package
  entrypoint, runtime adapters, and protocol wrappers
- [`sdk/typescript/generated`](../../sdk/typescript/generated): generated schema
  and protocol types
- [`sdk/typescript/examples`](../../sdk/typescript/examples): runnable examples
- [`sdk/typescript/tests`](../../sdk/typescript/tests): runtime and protocol
  regression tests
- [`sdk/typescript/README.md`](../../sdk/typescript/README.md): package-level
  quick start and validation commands
- [`README.md`](../../README.md): repo-wide setup and validation commands

## Public Surface Shape

- `src/index.ts` is the package entrypoint and re-exports the stable client
  factories, helpers, and protocol facade.
- `src/protocol.ts` is the handwritten SDK facade over the generated protocol
  schema in `sdk/typescript/generated`.
- `src/runtime/*_client.ts` contains the runtime-specific convenience helpers
  for Deno, Node, and Bun.

## Current Status

- Package name: `@meridian/cli-sdk`
- Source root: [`sdk/typescript/src`](../../sdk/typescript/src)
- Examples: [`sdk/typescript/examples`](../../sdk/typescript/examples)
- Today’s docs assume repo-local development first, with package-manager
  distribution documentation still to be added later
