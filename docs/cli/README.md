# CLI Guide

This section covers the curated Meridian CLI surface in
[`crates/meridian-cli`](../../crates/meridian-cli). Use it for local command
work, use `stdio` for custom automation, and use the TypeScript SDK when you
want a higher-level programmable integration.

The examples below assume your shell already has the required tools available.
On Linux, `nix-shell` is the easiest way to get that environment.

## Sections

- [Workflows](./workflows.md): analyze, inspect, process, and merge MIDI files
- [Rendering](./rendering.md): audio, video, and frame export examples
- [stdio Mode](./stdio.md): long-lived protocol mode and the raw `json` helper
- [Testing Guide](../contributor/testing.md): repo-local validation commands

## First Commands

```bash
cargo run -p meridian-cli -- --help
cargo run -p meridian-cli -- analyze song.mid --pretty
cargo run -p meridian-cli -- inspect song.mid --pretty
cargo run -p meridian-cli -- stdio
```

`--help` is the quickest command-surface check. `analyze` is the fastest
end-to-end parse check. `stdio` is the right starting point for another
process.

## Curated Posture

The CLI does not expose every internal protocol feature as a separate command.
That is deliberate.

- `analyze`, `inspect`, `process`, `merge`, and `render` cover the common local
  workflows.
- `stdio` is the stable automation surface for custom frontends and scripts.
- `json` is the raw one-shot protocol helper when you want to send a single
  request without keeping a process open.
- The TypeScript SDK is the preferred place for larger integrations that need a
  richer client API.

## Command Map

- `analyze`: inspect a single MIDI file and emit analysis JSON
- `inspect`: inspect one or more MIDI files
- `process`: modify MIDI files with curated subcommands
- `merge`: combine MIDI files into a single output
- `render audio`: render a MIDI file to WAV, FLAC, or MP3
- `render video`: render a MIDI file to MP4 or MKV
- `render frame`: write a single rendered frame to stdout
- `stdio`: run the line-delimited JSON protocol server
- `json`: send one raw protocol request and exit

## Reading Order

Recommended order:

1. [Workflows](./workflows.md)
2. [Rendering](./rendering.md)
3. [stdio Mode](./stdio.md)

This moves from the basic commands to rendering and then to automation.
