# meridian

Meridian is a Rust workspace for MIDI inspection, modification, audio rendering,
and desktop visualization.

## Start Here

- [Getting started](docs/getting-started.md): the fastest path to a first
  successful CLI, UI, or SDK workflow
- [Repository docs](docs/README.md): the full documentation index
- [CLI guide](docs/cli/README.md): command-line workflows from simple to
  advanced
- [UI guide](docs/ui/README.md): panel-by-panel UI workflows and export paths
- [TypeScript SDK docs](docs/typescript-sdk/README.md): SDK setup, runtime
  support, and API surface notes
- [Workspace layout](#workspace): where the core crates, CLI, UI, and SDK live
- [Validation matrix](#validation-matrix): which commands actually exercise
  which parts of the repo

## Workspace

- [`crates/meridian-core`](crates/meridian-core): core MIDI, rendering,
  protocol, and job logic
- [`crates/meridian-cli`](crates/meridian-cli): CLI and stdio protocol frontend
- [`crates/meridian-ui`](crates/meridian-ui): Slint desktop UI
- [`sdk/typescript`](sdk/typescript): TypeScript SDK for driving `meridian-cli`
  via the public entrypoint in `sdk/typescript/src/index.ts`

## Docs

- [Repository docs](docs/README.md)
- [Getting started](docs/getting-started.md): first-run setup and smoke paths
- [CLI guide](docs/cli/README.md): curated CLI workflows and advanced options
- [UI guide](docs/ui/README.md): playback, panels, export, and persistence
- [Troubleshooting](docs/troubleshooting.md): common setup and runtime failures
- [Contributor docs](docs/contributor/README.md): contributor map and validation
  entrypoints

## Requirements

- Rust toolchain compatible with [`rust-toolchain.toml`](rust-toolchain.toml)
- On Linux, native libraries for ALSA, fontconfig, X11/Wayland, and
  OpenGL/Vulkan
- The included [`shell.nix`](shell.nix) supplies the native Linux libraries,
  `ffmpeg`, and `deno` used by the build, rendering, and SDK workflows
- A real desktop session for running the UI; headless SSH sessions can still
  build and run non-UI paths

On Linux, the safest default is to run Cargo and Deno commands through
`nix-shell`.

If you use `direnv`, run `direnv allow` once in this repo. The included
[`.envrc`](.envrc) loads the `shell.nix` environment and keeps the host Rust
toolchain preferred.

## Validation Matrix

These are the commands most useful for newcomer validation. They are grouped by
the code surfaces they actually exercise.

```bash
nix-shell --run 'cargo check --workspace'
```

Checks all workspace crates.

```bash
nix-shell --run 'cargo test -p meridian-core --lib'
```

Runs the core unit test suite.

```bash
nix-shell --run 'cargo test -p meridian-core --test core_flow'
```

Runs the core integration flow tests, including render and process smoke
coverage.

```bash
nix-shell --run 'cargo test -p meridian-cli --tests'
```

Runs the CLI integration and stdio tests.

```bash
nix-shell --run 'cargo test -p meridian-ui'
```

Runs the UI unit and integration tests.

```bash
nix-shell --run 'cd sdk/typescript && deno test --allow-env --allow-read --allow-write --allow-run tests'
```

Runs the full TypeScript SDK runtime test suite.

## Common Commands

Run the CLI help:

```bash
nix-shell --run 'cargo run -p meridian-cli -- --help'
```

Run CLI stdio protocol mode:

```bash
nix-shell --run 'cargo run -p meridian-cli -- stdio'
```

Analyze a MIDI file:

```bash
nix-shell --run 'cargo run -p meridian-cli -- analyze song.mid --pretty --buckets 64'
```

Inspect one or more MIDI files:

```bash
nix-shell --run 'cargo run -p meridian-cli -- inspect song.mid other.mid --pretty'
```

Merge MIDI files:

```bash
nix-shell --run 'cargo run -p meridian-cli -- merge left.mid right.mid --output merged.mid --pretty'
```

Process a MIDI file:

```bash
nix-shell --run 'cargo run -p meridian-cli -- process select song.mid --output excerpt.mid --start-ticks 0 --end-ticks 1920 --pretty'
```

Render audio:

```bash
nix-shell --run 'cargo run -p meridian-cli -- render audio song.mid --output out.flac --format flac'
```

Render video with a custom time range:

```bash
nix-shell --run 'cargo run -p meridian-cli -- render video song.mid --output clip.mp4 --start-time 12.5 --end-time 18.0 --fps 30'
```

Run the desktop UI:

```bash
nix-shell --run 'cargo run -p meridian-ui'
```

Run the UI without the accelerated viewport:

```bash
nix-shell --run 'MERIDIAN_DISABLE_WGPU=1 cargo run -p meridian-ui'
```

Run TypeScript SDK checks:

```bash
nix-shell --run 'cd sdk/typescript && deno check src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts tests/client_internal_test.ts'
```

## Notes

- The repo is self-contained at the Cargo level and uses the published
  `midi-toolkit-rs` crate rather than requiring a sibling checkout.
- The default audio configuration can use the embedded soundfont, or you can
  override it with `MERIDIAN_SOUNDFONT`.
- Video and encoded audio rendering rely on `ffmpeg` being available on the
  host.
- The UI needs a display server; expected headless failures are environmental,
  not compile failures.
- The TypeScript SDK docs, examples, and runtime tests now use the same checked
  Deno workflow.

## Licensing

Meridian uses a mixed licensing model:

- [`meridian-core`](crates/meridian-core/Cargo.toml) is licensed under
  `LGPL-3.0-or-later`.
- [`meridian-cli`](crates/meridian-cli/Cargo.toml) is licensed under
  `GPL-3.0-only`.
- [`meridian-ui`](crates/meridian-ui/Cargo.toml) is licensed under
  `GPL-3.0-only`.

See [LICENSE.md](LICENSE.md) for the repo-level summary and bundled license
texts.
