# God File Prototype

Worktree: `/home/arduano/programming/meridian-god-file-splits`
Branch: `prototype/god-file-splits`

Rule used for this prototype:
- Count logic files over 500 LOC.
- Ignore `.slint`.
- Ignore test modules when deciding whether a file is a logic god file.

## Split

- `crates/meridian-ui/src/ui/runtime.rs`
  - Split into focused runtime modules.
  - Current top-level file is 80 LOC.

- `crates/meridian-ui/src/ui/state.rs`
  - Split into domain modules under `crates/meridian-ui/src/ui/state/`.

- `sdk/typescript/src/internal/client.ts`
  - Split into focused client modules under `sdk/typescript/src/internal/client/`.

- `crates/meridian-core/src/engine/resources.rs`
  - Split into focused resource modules under `crates/meridian-core/src/engine/resources/`.

- `crates/meridian-core/src/midi/analysis.rs`
  - Split into focused analysis modules under `crates/meridian-core/src/midi/analysis/`.

- `crates/meridian-cli/src/cli.rs`
  - Split into focused CLI modules under `crates/meridian-cli/src/cli/`.

- `crates/meridian-core/src/protocol/wire.rs`
  - Split protocol conversion impls into `crates/meridian-core/src/protocol/wire/conversions.rs`.
  - Current top-level file is 288 LOC.

- `crates/meridian-core/src/render/shared/config.rs`
  - Split default-value helpers into `crates/meridian-core/src/render/shared/config/defaults.rs`.
  - Current top-level file is 480 LOC.

## Keep

- `crates/meridian-core/src/midi/file_merge.rs`
  - 697 total LOC, but 477 logic LOC before `#[cfg(test)]`.
  - Not a logic god file under the agreed rule.

- `crates/meridian-core/src/midi/modifier_tools/shared_metadata_track.rs`
  - 940 total LOC, but 476 logic LOC before `#[cfg(test)]`.
  - Not a logic god file under the agreed rule.

- `crates/meridian-core/src/render/piano_trail_classic/projector.rs`
  - 1077 total LOC, 1019 logic LOC before tests.
  - Kept for now.
  - Justification: this is a single performance-sensitive geometry projection pipeline with tightly coupled note visibility, camera transforms, keyboard emission, aura emission, and quad packing. A mechanical split here would mostly move hot-path math and mutable buffer writes across files without reducing conceptual complexity much, and it has higher render-regression risk than the other refactors in this prototype.

## Verification

- `cargo fmt --all` passed.
- `cargo check -p meridian-core --lib` is blocked by missing system `alsa` development files (`alsa.pc`).
- `cargo check -p meridian-ui` is blocked by missing system `fontconfig` development files (`fontconfig.pc`).
