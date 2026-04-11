# meridian

Meridian is a Rust workspace for MIDI inspection, modification, audio rendering, and desktop visualization.

## Workspace

- [`crates/meridian-core`](/home/arduano/programming/meridian/crates/meridian-core): core MIDI, rendering, protocol, and job logic
- [`crates/meridian-cli`](/home/arduano/programming/meridian/crates/meridian-cli): CLI and stdio protocol frontend
- [`crates/meridian-ui`](/home/arduano/programming/meridian/crates/meridian-ui): Slint desktop UI
- [`sdk/typescript`](/home/arduano/programming/meridian/sdk/typescript): TypeScript SDK for driving `meridian-cli`

## Requirements

- Rust toolchain compatible with [`rust-toolchain.toml`](/home/arduano/programming/meridian/rust-toolchain.toml)
- On Linux, native libraries for ALSA, fontconfig, X11/Wayland, and OpenGL/Vulkan
- `ffmpeg` for encoded audio/video rendering and related smoke tests
- A real desktop session for running the UI; headless SSH sessions can still build and run non-UI paths

The included [`shell.nix`](/home/arduano/programming/meridian/shell.nix) supplies the native Linux libraries expected by the Rust crates. On Linux, the safest default is to run Cargo commands through `nix-shell`.

If you use `direnv`, run `direnv allow` once in this repo. The included [`.envrc`](/home/arduano/programming/meridian/.envrc) loads the `shell.nix` environment and keeps the host Rust toolchain preferred.

## Common Commands

Check the workspace:

```bash
nix-shell --run 'cargo check'
```

Run core library tests:

```bash
nix-shell --run 'cargo test -p meridian-core --lib'
```

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

Process a MIDI file:

```bash
nix-shell --run "cargo run -p meridian-cli -- process select song.mid --output right-hand.mid --key-min 60 --event-kind note --pretty"
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
nix-shell --run 'cd sdk/typescript && deno check src/index.ts tests/sdk_test.ts tests/protocol_stdio_test.ts'
```

## Notes

- The repo is self-contained at the Cargo level and uses the published `midi-toolkit-rs` crate rather than requiring a sibling checkout.
- The default audio configuration can use the embedded soundfont, or you can override it with `MERIDIAN_SOUNDFONT`.
- Video and encoded audio rendering rely on `ffmpeg` being available on the host.
- The UI needs a display server; expected headless failures are environmental, not compile failures.

## Licensing

Meridian uses a mixed licensing model:

- [`meridian-core`](/home/arduano/programming/meridian/crates/meridian-core/Cargo.toml) is licensed under `LGPL-3.0-or-later`.
- [`meridian-cli`](/home/arduano/programming/meridian/crates/meridian-cli/Cargo.toml) is licensed under `GPL-3.0-only`.
- [`meridian-ui`](/home/arduano/programming/meridian/crates/meridian-ui/Cargo.toml) is licensed under `GPL-3.0-only`.

See [LICENSE.md](/home/arduano/programming/meridian/LICENSE.md) for the repo-level summary and bundled license texts.
