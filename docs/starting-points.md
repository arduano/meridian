# Starting Points

This is a contributor map, not a full architecture spec. Use it when you want to
find the right file quickly.

## Core

- Request and state orchestration: [`crates/meridian-core/src/engine/mod.rs`](../crates/meridian-core/src/engine/mod.rs)
- Core state mutation and rollback: [`crates/meridian-core/src/engine/core_state.rs`](../crates/meridian-core/src/engine/core_state.rs)
- MIDI domain model and file helpers: [`crates/meridian-core/src/midi/mod.rs`](../crates/meridian-core/src/midi/mod.rs)
- MIDI processing and modifier tools: [`crates/meridian-core/src/midi/processing.rs`](../crates/meridian-core/src/midi/processing.rs), [`crates/meridian-core/src/midi/modifier_tools/mod.rs`](../crates/meridian-core/src/midi/modifier_tools/mod.rs)
- Video render orchestration: [`crates/meridian-core/src/video/mod.rs`](../crates/meridian-core/src/video/mod.rs)
- Audio render orchestration: [`crates/meridian-core/src/audio/render/mod.rs`](../crates/meridian-core/src/audio/render/mod.rs)
- Shared render types and scene config: [`crates/meridian-core/src/render/mod.rs`](../crates/meridian-core/src/render/mod.rs), [`crates/meridian-core/src/render/shared/config.rs`](../crates/meridian-core/src/render/shared/config.rs)

## UI

- UI entry and runtime bootstrap: [`crates/meridian-ui/src/ui/mod.rs`](../crates/meridian-ui/src/ui/mod.rs), [`crates/meridian-ui/src/main.rs`](../crates/meridian-ui/src/main.rs)
- Shared UI state and reducer/projection boundary: [`crates/meridian-ui/src/ui/state.rs`](../crates/meridian-ui/src/ui/state.rs), [`crates/meridian-ui/src/ui/state/reduce.rs`](../crates/meridian-ui/src/ui/state/reduce.rs), [`crates/meridian-ui/src/ui/state/apply.rs`](../crates/meridian-ui/src/ui/state/apply.rs), [`crates/meridian-ui/src/ui/view_model.rs`](../crates/meridian-ui/src/ui/view_model.rs)
- Modify panel flow: [`crates/meridian-ui/src/ui/runtime/panels/modify_registry.rs`](../crates/meridian-ui/src/ui/runtime/panels/modify_registry.rs)
- Merge panel flow: [`crates/meridian-ui/src/ui/runtime/panels/merge.rs`](../crates/meridian-ui/src/ui/runtime/panels/merge.rs)
- Render export setup and lifecycle: [`crates/meridian-ui/src/ui/runtime/export.rs`](../crates/meridian-ui/src/ui/runtime/export.rs), [`crates/meridian-ui/src/ui/runtime/render_export/controller.rs`](../crates/meridian-ui/src/ui/runtime/render_export/controller.rs)
- Video display state and controls: [`crates/meridian-ui/src/ui/state/video.rs`](../crates/meridian-ui/src/ui/state/video.rs), [`crates/meridian-ui/src/ui/runtime/video.rs`](../crates/meridian-ui/src/ui/runtime/video.rs)

## CLI

- Canonical command dispatcher: [`crates/meridian-cli/src/cli.rs`](../crates/meridian-cli/src/cli.rs)
- Binary entrypoint and legacy aliases: [`crates/meridian-cli/src/main.rs`](../crates/meridian-cli/src/main.rs)
- Help and surface tests: [`crates/meridian-cli/tests/cli_help.rs`](../crates/meridian-cli/tests/cli_help.rs)
- Stdio protocol smoke tests: [`crates/meridian-cli/tests/cli_stdio.rs`](../crates/meridian-cli/tests/cli_stdio.rs)
- Render and merge command tests: [`crates/meridian-cli/tests/cli_render.rs`](../crates/meridian-cli/tests/cli_render.rs), [`crates/meridian-cli/tests/cli_merge_inspect.rs`](../crates/meridian-cli/tests/cli_merge_inspect.rs)

## TypeScript SDK

- Package entrypoint: [`sdk/typescript/src/index.ts`](../sdk/typescript/src/index.ts)
- Protocol facade and public type aliases: [`sdk/typescript/src/protocol.ts`](../sdk/typescript/src/protocol.ts)
- Runtime adapters: [`sdk/typescript/src/runtime/deno_client.ts`](../sdk/typescript/src/runtime/deno_client.ts), [`sdk/typescript/src/runtime/node_client.ts`](../sdk/typescript/src/runtime/node_client.ts), [`sdk/typescript/src/runtime/bun_client.ts`](../sdk/typescript/src/runtime/bun_client.ts)
- SDK examples: [`sdk/typescript/examples`](../sdk/typescript/examples)
- SDK tests: [`sdk/typescript/tests/sdk_test.ts`](../sdk/typescript/tests/sdk_test.ts), [`sdk/typescript/tests/protocol_stdio_test.ts`](../sdk/typescript/tests/protocol_stdio_test.ts), [`sdk/typescript/tests/client_internal_test.ts`](../sdk/typescript/tests/client_internal_test.ts)

## Rendering

- Render scene and shared config types: [`crates/meridian-core/src/render/mod.rs`](../crates/meridian-core/src/render/mod.rs)
- Flat, PFA, text, and piano-trail-classic renderers: [`crates/meridian-core/src/render/flat/mod.rs`](../crates/meridian-core/src/render/flat/mod.rs), [`crates/meridian-core/src/render/pfa/mod.rs`](../crates/meridian-core/src/render/pfa/mod.rs), [`crates/meridian-core/src/render/text/mod.rs`](../crates/meridian-core/src/render/text/mod.rs), [`crates/meridian-core/src/render/piano_trail_classic/mod.rs`](../crates/meridian-core/src/render/piano_trail_classic/mod.rs)
- Headless export support: [`crates/meridian-core/src/render/export.rs`](../crates/meridian-core/src/render/export.rs), [`crates/meridian-core/src/render/headless.rs`](../crates/meridian-core/src/render/headless.rs)

## Tests

- Core integration and smoke tests: [`crates/meridian-core/tests/core_flow.rs`](../crates/meridian-core/tests/core_flow.rs)
- Core focused regression tests: [`crates/meridian-core/tests/analysis_polyphony.rs`](../crates/meridian-core/tests/analysis_polyphony.rs), [`crates/meridian-core/tests/pfa_config.rs`](../crates/meridian-core/tests/pfa_config.rs)
- UI regression tests: [`crates/meridian-ui/src/ui/runtime/persistence/tests.rs`](../crates/meridian-ui/src/ui/runtime/persistence/tests.rs)
- CLI/stdio protocol tests: [`crates/meridian-cli/tests/cli_stdio.rs`](../crates/meridian-cli/tests/cli_stdio.rs)
- SDK/runtime tests: [`sdk/typescript/tests/sdk_test.ts`](../sdk/typescript/tests/sdk_test.ts), [`sdk/typescript/tests/protocol_stdio_test.ts`](../sdk/typescript/tests/protocol_stdio_test.ts)

## Good First Commands

These are the first commands I would run when changing the repo:

```bash
nix-shell --run 'cargo test -p meridian-core --lib'
nix-shell --run 'cargo test -p meridian-cli --tests'
nix-shell --run 'cargo test -p meridian-ui persistence::tests -- --nocapture'
cd sdk/typescript && deno test --allow-env --allow-read --allow-write --allow-run tests
```

If you are changing render-heavy code, add the relevant focused smoke test from
`core_flow.rs` or `cli_stdio.rs`.
