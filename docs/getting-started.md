# Getting Started

Use this page to get Meridian running and verify a first UI path. CLI and SDK
paths follow after that.

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

The command examples below assume you are already in a shell with the required
tools available. On Linux, that usually means running them from `nix-shell` or
an equivalent environment.

## 2. Confirm The Workspace Builds

Run the broad compile check first:

```bash
cargo check --workspace
```

If that passes, you have a usable baseline environment.

## 3. First Successful UI Launch

Run the desktop UI:

```bash
cargo run -p meridian-ui
```

If the accelerated viewport cannot start on your host, use:

```bash
cargo run -p meridian-ui -- --disable-wgpu
```

The simplest UI smoke path is:

1. launch the app
2. load a MIDI file
3. press play
4. drag the time slider
5. open the `Video` or `Render` tab

See [UI Guide](./ui/README.md) for the rest of the UI workflows.

## 4. First Successful CLI Command

Pick any MIDI file and run:

```bash
cargo run -p meridian-cli -- analyze song.mid --pretty --buckets 32
```

If you want a few more immediate workflows after that:

- inspect MIDI metadata:

```bash
cargo run -p meridian-cli -- inspect song.mid --pretty
```

- render audio:

```bash
cargo run -p meridian-cli -- render audio song.mid --output out.flac --format flac
```

- render video:

```bash
cargo run -p meridian-cli -- render video song.mid --output clip.mp4 --start-time -1 --end-time 8 --fps 30
```

See [CLI Guide](./cli/README.md) for the rest of the command surface.

## 5. TypeScript SDK Quick Start

If you want to drive Meridian programmatically, start here:

- [TypeScript SDK package README](../sdk/typescript/README.md)
- [TypeScript SDK docs](./typescript-sdk/README.md)

The checked repo-local SDK workflow is:

```bash
cd sdk/typescript
deno check mod.ts src/index.ts src/runtime/deno_client.ts examples/*.ts tests/common.ts tests/sdk_test.ts tests/protocol_stdio_test.ts tests/client_internal_test.ts
deno test --allow-env --allow-read --allow-write --allow-run tests
```

## 6. Next Docs

- [UI Guide](./ui/README.md): panel-by-panel UI workflows
- [Render](./ui/render.md): export modes, ranges, and stream settings
- [Modify](./ui/modify.md): MIDI transformation controls
- [CLI Guide](./cli/README.md): command-line workflows for power users
- [Troubleshooting](./troubleshooting.md): setup and runtime failure modes
- [Contributor Docs](./contributor/README.md): codebase map and validation entrypoints
