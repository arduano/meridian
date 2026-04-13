# Getting Started

This guide is the fastest way to get one successful Meridian workflow running.
It is aimed at someone who wants to use the project first and read architecture
docs later.

## 1. Build Environment

On Linux, the safest default is to use the repo's Nix shell:

```bash
nix-shell
```

That shell provides the native libraries and tools the repo expects, including:

- Rust build dependencies
- `ffmpeg`
- `deno`

If you use `direnv`, run:

```bash
direnv allow
```

## 2. Confirm The Workspace Builds

Run the broad compile check first:

```bash
nix-shell --run 'cargo check --workspace'
```

If that passes, you have a usable baseline environment.

## 3. First Successful CLI Command

Pick any MIDI file and run:

```bash
nix-shell --run 'cargo run -p meridian-cli -- analyze song.mid --pretty --buckets 32'
```

If you want a few more immediate workflows after that:

- inspect MIDI metadata:

```bash
nix-shell --run 'cargo run -p meridian-cli -- inspect song.mid --pretty'
```

- render audio:

```bash
nix-shell --run 'cargo run -p meridian-cli -- render audio song.mid --output out.flac --format flac'
```

- render video:

```bash
nix-shell --run 'cargo run -p meridian-cli -- render video song.mid --output clip.mp4 --start-time -1 --end-time 8 --fps 30'
```

For the full command progression, continue to [CLI Guide](./cli/README.md).

## 4. First Successful UI Launch

Run the desktop UI:

```bash
nix-shell --run 'cargo run -p meridian-ui'
```

If the accelerated viewport cannot start on your host, use:

```bash
nix-shell --run 'MERIDIAN_DISABLE_WGPU=1 cargo run -p meridian-ui'
```

The simplest UI smoke path is:

1. launch the app
2. load a MIDI file
3. press play
4. drag the time slider
5. save a frame or open the export panel

For the full panel-by-panel walkthrough, continue to [UI Guide](./ui/README.md).

## 5. TypeScript SDK Quick Start

If you want to drive Meridian programmatically, start here:

- [TypeScript SDK package README](../sdk/typescript/README.md)
- [TypeScript SDK docs](./typescript-sdk/README.md)

The checked repo-local SDK workflow is:

```bash
nix-shell --run 'cd sdk/typescript && deno check src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts tests/client_internal_test.ts'
nix-shell --run 'cd sdk/typescript && deno test --allow-env --allow-read --allow-write --allow-run tests'
```

## 6. Where To Go Next

- [CLI Guide](./cli/README.md): user-facing commands from quickstart to advanced flags
- [UI Guide](./ui/README.md): panels, playback, export, and persistence behavior
- [Troubleshooting](./troubleshooting.md): setup and runtime failure modes
- [Contributor Docs](./contributor/README.md): codebase map and validation entrypoints
