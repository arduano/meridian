# meridian

Meridian is a Rust workspace for MIDI inspection, modification, audio rendering, and desktop visualization.

## Start Here

- [Repository docs](docs/README.md): the top-level docs index and the best place to find feature-specific guides
- [TypeScript SDK docs](docs/typescript-sdk/README.md): SDK setup, runtime support, and API surface notes
- [Workspace layout](#workspace): where the core crates, CLI, UI, and SDK live
- [Validation matrix](#validation-matrix): which commands actually exercise which parts of the repo

## Workspace

- [`crates/meridian-core`](crates/meridian-core): core MIDI, rendering, protocol, and job logic
- [`crates/meridian-cli`](crates/meridian-cli): CLI and stdio protocol frontend
- [`crates/meridian-ui`](crates/meridian-ui): Slint desktop UI
- [`sdk/typescript`](sdk/typescript): TypeScript SDK for driving `meridian-cli`
  via the public entrypoint in `sdk/typescript/src/index.ts`

## Docs

- [Repository docs](docs/README.md)
- [Starting points](docs/starting-points.md): contributor map for core, UI,
  CLI, SDK, rendering, and tests

## Requirements

- Rust toolchain compatible with [`rust-toolchain.toml`](rust-toolchain.toml)
- On Linux, native libraries for ALSA, fontconfig, X11/Wayland, and OpenGL/Vulkan
- The included [`shell.nix`](shell.nix) supplies the native Linux libraries, `ffmpeg`, and `deno` used by the build, rendering, and SDK workflows
- A real desktop session for running the UI; headless SSH sessions can still build and run non-UI paths

On Linux, the safest default is to run Cargo and Deno commands through `nix-shell`.

If you use `direnv`, run `direnv allow` once in this repo. The included [`.envrc`](.envrc) loads the `shell.nix` environment and keeps the host Rust toolchain preferred.

## Validation Matrix

These are the commands most useful for newcomer validation. They are grouped by the code surfaces they actually exercise.

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

Runs the core integration flow tests, including render and process smoke coverage.

```bash
nix-shell --run 'cargo test -p meridian-cli --tests'
```

Runs the CLI integration and stdio tests.

```bash
nix-shell --run 'cargo test -p meridian-ui'
```

Runs the UI unit and integration tests.

```bash
nix-shell --run 'cd sdk/typescript && deno test --no-check --allow-env --allow-read --allow-write --allow-run tests'
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
nix-shell --run 'cd sdk/typescript && deno check src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts'
```

## Notes

- The repo is self-contained at the Cargo level and uses the published `midi-toolkit-rs` crate rather than requiring a sibling checkout.
- The default audio configuration can use the embedded soundfont, or you can override it with `MERIDIAN_SOUNDFONT`.
- Video and encoded audio rendering rely on `ffmpeg` being available on the host.
- The UI needs a display server; expected headless failures are environmental, not compile failures.
- The TypeScript SDK newcomer validation path is currently split between `deno check ...` for the documented entrypoints and `deno test --no-check ...` for the runtime suite.

## Licensing

Meridian uses a mixed licensing model:

- [`meridian-core`](crates/meridian-core/Cargo.toml) is licensed under `LGPL-3.0-or-later`.
- [`meridian-cli`](crates/meridian-cli/Cargo.toml) is licensed under `GPL-3.0-only`.
- [`meridian-ui`](crates/meridian-ui/Cargo.toml) is licensed under `GPL-3.0-only`.

See [LICENSE.md](LICENSE.md) for the repo-level summary and bundled license texts.
