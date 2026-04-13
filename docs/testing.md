# Testing Guide

This guide maps common change areas to the smallest useful validation commands.

## First Pass

Run this when you want a broad compile sanity check:

```bash
nix-shell --run 'cargo check --workspace'
```

## Core

- Unit-heavy core changes:

```bash
nix-shell --run 'cargo test -p meridian-core --lib'
```

- Integration flow and render/process smoke coverage:

```bash
nix-shell --run 'cargo test -p meridian-core --test core_flow'
```

- Focused MIDI analysis regressions:

```bash
nix-shell --run 'cargo test -p meridian-core --test analysis_polyphony'
```

## CLI

- Help and command-surface regressions:

```bash
nix-shell --run 'cargo test -p meridian-cli --test cli_help'
```

- Curated command workflows:

```bash
nix-shell --run 'cargo test -p meridian-cli --test cli_curated_commands'
```

- Stdio protocol and render smoke:

```bash
nix-shell --run 'cargo test -p meridian-cli --test cli_stdio'
```

## UI

- Broad UI compile sanity:

```bash
nix-shell --run 'cargo check -p meridian-ui'
```

- Persistence-specific regressions:

```bash
nix-shell --run 'cargo test -p meridian-ui persistence::tests -- --nocapture'
```

- Export and viewport regressions:

```bash
nix-shell --run 'cargo test -p meridian-ui viewport::tests -- --nocapture'
```

## TypeScript SDK

- Type-check the public surface, examples, and tests:

```bash
nix-shell --run 'cd sdk/typescript && deno check src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts tests/client_internal_test.ts'
```

- Run the runtime suite:

```bash
nix-shell --run 'cd sdk/typescript && deno test --allow-env --allow-read --allow-write --allow-run tests'
```

## Notes

- UI test binaries are the heaviest native builds in the workspace. If memory is
  tight, prefer one focused UI test target at a time.
- Video render tests require `ffmpeg`.
- Audio render tests require a usable soundfont path.
